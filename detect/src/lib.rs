//! Runs what langbank describes.
//!
//! The leaf crate states facts and executes nothing: extensions, shebangs, and
//! 317 ordered content rules for the extensions a name cannot settle. Somebody
//! has to run them, and until this crate existed nobody ever had — the rules
//! were asserted to work and never checked.
//!
//! This is deliberately a separate crate. It needs a regex engine and langbank
//! must not, so a consumer that only wants to know what a `.rs` file is pays
//! nothing for the machinery that reads one.

use std::sync::OnceLock;

use langbank::{
    Disambiguation, LanguageProfile, disambiguation_for, language_profile_for_extension,
};
use regex::Regex;

/// How a language was decided, which matters as much as the answer: a name is
/// cheap and sometimes wrong, and reading the file is neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    /// The filename identified it outright.
    Filename(String),
    /// The extension identified it, uncontested or won outright.
    Extension(String),
    /// A `#!` line named an interpreter.
    Shebang(String),
    /// Several languages claim the extension and a content rule settled it.
    Content { extension: String, rule: usize },
}

#[derive(Debug, Clone)]
pub struct Identification {
    pub language: &'static LanguageProfile,
    pub evidence: Evidence,
}

/// Why nothing was decided. Absence with a reason, as the registry has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undecided {
    /// No language claims anything about this name.
    Unknown,
    /// Several claim it, and no rule matched — or there were no rules.
    Contested {
        extension: String,
        claimants: Vec<&'static str>,
        had_rules: bool,
    },
}

/// Take a lock, recovering from poisoning rather than propagating it.
///
/// The regex cache holds no invariant a panic could have broken — it is a map
/// from a pattern to a compiled copy of that pattern, and a half-finished
/// insert leaves it merely incomplete. Treating a poisoned lock as failure
/// instead meant that one panic anywhere while it was held turned every content
/// rule off for the life of the process: silently and permanently, with
/// `identify` falling back to filename answers and reporting nothing wrong. On
/// a long-running server that is a site that quietly stops working and still
/// returns 200 — the same failure shape as the `^`-anchoring bug, and just as
/// hard to notice.
///
/// Only reachable if something panics inside the critical section below, which
/// is why `lock_poisoning_is_recovered_from` poisons a mutex directly rather
/// than trying to provoke it through `identify` — an earlier attempt that did
/// the latter poisoned nothing and passed against the unfixed code.
fn lock_recovering<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn compiled(pattern: &str) -> Option<&'static Regex> {
    // Rules are evaluated repeatedly and the patterns are fixed, so each is
    // compiled once. Six of 317 use a construct Rust's regex rejects; those
    // simply never match rather than taking the process down.
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, &'static Regex>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = lock_recovering(cache);
    if let Some(found) = cache.get(pattern) {
        return Some(found);
    }
    // Linguist's patterns are Ruby's, where `^` and `$` match at every line
    // boundary — there is no single-line mode to turn off. Rust's regex anchors
    // them to the whole haystack unless multi-line is asked for, so a rule like
    // `^\s*template\s*<` would never fire on a file whose second line matches.
    // Every one of these patterns is a search within a file, so this is on.
    let compiled: &'static Regex = Box::leak(Box::new(
        regex::RegexBuilder::new(pattern)
            .multi_line(true)
            .build()
            // Discarding this one is deliberate: a pattern that does not
            // compile is a rule that cannot fire, which langbank already
            // records in the data as `portable = false`, and
            // `every_rule_in_the_registry_compiles_or_is_marked_unportable`
            // fails if the two disagree. `identify` answers `Undecided`, which
            // is a statement about the file rather than about the registry, so
            // there is nothing here for a caller to do with the error.
            .ok()?,
    ));
    cache.insert(pattern.to_owned(), compiled);
    Some(compiled)
}

fn holds(rule: &langbank::DisambiguationRule, content: &str) -> bool {
    rule.clauses.iter().all(|clause| {
        let positive = clause.patterns.is_empty()
            || clause
                .patterns
                .iter()
                .any(|pattern| compiled(pattern).is_some_and(|regex| regex.is_match(content)));
        let negative = clause
            .negative
            .iter()
            .all(|pattern| compiled(pattern).is_none_or(|regex| !regex.is_match(content)));
        positive && negative
    })
}

/// Apply an extension's ordered rules to some content. First rule whose clauses
/// all hold wins, which is exactly what the data says.
pub fn apply(block: &Disambiguation, content: &str) -> Option<(&'static LanguageProfile, usize)> {
    block
        .rules
        .iter()
        .enumerate()
        .find(|(_, rule)| holds(rule, content))
        .map(|(index, rule)| (rule.language, index))
}

fn extension_of(name: &str) -> Option<String> {
    let base = name.rsplit('/').next()?;
    let (_, extension) = base.rsplit_once('.')?;
    (!extension.is_empty()).then(|| extension.to_ascii_lowercase())
}

/// Identify a file from its path, and its content where the path is not enough.
///
/// Pass `None` for content to get the name-only answer — which is what a
/// walker that has not opened the file yet can afford.
pub fn identify(path: &str, content: Option<&str>) -> Result<Identification, Undecided> {
    let name = path.rsplit('/').next().unwrap_or(path);

    // Reading the file beats guessing from its name. `.h` resolves to C by a
    // primary claim, which is the right answer for a walker that has not opened
    // the file — and the wrong one for a caller holding the bytes, because
    // linguist publishes rules that tell C from C++ from Objective-C. So the
    // rules go first whenever there is content to run them against.
    if let Some(content) = content
        && let Some(extension) = extension_of(path)
        && langbank::languages_claiming_extension(&extension).len() > 1
        && let Some(block) = disambiguation_for(&extension)
        && let Some((language, rule)) = apply(block, content)
    {
        return Ok(Identification {
            language,
            evidence: Evidence::Content { extension, rule },
        });
    }

    if let Some(detection) = langbank::detect_language(std::path::Path::new(path), None) {
        if let Some(language) = langbank::language_profile(detection.language.as_str()) {
            let evidence = match detection.evidence.first() {
                Some(langbank::LanguageEvidence::Filename { filename }) => {
                    Evidence::Filename(filename.clone())
                }
                Some(langbank::LanguageEvidence::Extension { extension }) => {
                    Evidence::Extension(extension.clone())
                }
                _ => Evidence::Filename(name.to_owned()),
            };
            return Ok(Identification { language, evidence });
        }
    }

    if let Some(content) = content
        && let Some(detection) =
            langbank::detect_language(std::path::Path::new(path), Some(content.as_bytes()))
        && let Some(language) = langbank::language_profile(detection.language.as_str())
    {
        let interpreter = match detection.evidence.first() {
            Some(langbank::LanguageEvidence::Shebang { interpreter }) => interpreter.clone(),
            _ => String::new(),
        };
        return Ok(Identification {
            language,
            evidence: Evidence::Shebang(interpreter),
        });
    }

    let Some(extension) = extension_of(path) else {
        return Err(Undecided::Unknown);
    };
    let claimants = langbank::languages_claiming_extension(&extension);
    if claimants.is_empty() {
        return Err(Undecided::Unknown);
    }
    if let Some(only) = language_profile_for_extension(&extension) {
        return Ok(Identification {
            language: only,
            evidence: Evidence::Extension(extension),
        });
    }

    let block = disambiguation_for(&extension);
    if let (Some(block), Some(content)) = (block, content)
        && let Some((language, rule)) = apply(block, content)
    {
        return Ok(Identification {
            language,
            evidence: Evidence::Content { extension, rule },
        });
    }
    Err(Undecided::Contested {
        extension,
        claimants: claimants.iter().map(|profile| profile.id).collect(),
        had_rules: block.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::lock_recovering;
    use std::sync::Mutex;

    #[test]
    fn lock_poisoning_is_recovered_from() {
        let mutex = Mutex::new(vec!["a rule"]);
        let poisoned = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().expect("first lock");
            panic!("poison it while the guard is held");
        });
        assert!(poisoned.is_err(), "the panic did not happen");
        assert!(mutex.is_poisoned(), "the mutex was not actually poisoned");

        // The behaviour that matters: still usable, and the contents survived.
        // `mutex.lock().ok()` here would be `None`, which is how every content
        // rule used to switch itself off.
        assert!(mutex.lock().is_err(), "a plain lock would still fail");
        assert_eq!(*lock_recovering(&mutex), vec!["a rule"]);
    }
}
