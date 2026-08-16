//! Contested extensions, settled only where two independent corpora agree.
//!
//! Ported from `tools/resolve-contested.py`. An extension claimed by several
//! languages resolves to nothing unless exactly one claimant declares it
//! primary. Some of those are unresolved only because nobody has looked, and
//! tokei and scc have looked — so where both name the same claimant, that is
//! corroboration and the claim is written; where only one does, or the two
//! disagree, nothing is written and the reason is recorded as a gap.
//!
//! Detection declining is a fact about the data, not a bug to be papered over
//! by picking whichever corpus was read last.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const TOKEI: &str = "https://raw.githubusercontent.com/XAMPPRocky/tokei/master/languages.json";
const SCC: &str = "https://raw.githubusercontent.com/boyter/scc/master/languages.json";

fn slug(name: &str) -> String {
    let lowered = name
        .to_lowercase()
        .replace('#', "-sharp")
        .replace("++", "pp")
        .replace('*', "-star");
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        out.push(if c.is_alphanumeric() { c } else { '-' });
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

/// extension -> the languages a corpus says own it.
fn owners(corpus: &serde_json::Map<String, Value>) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, entry) in corpus {
        let Some(extensions) = entry.get("extensions").and_then(Value::as_array) else {
            continue;
        };
        for extension in extensions.iter().filter_map(Value::as_str) {
            out.entry(extension.to_lowercase().trim_start_matches('.').to_string())
                .or_default()
                .insert(slug(name));
        }
    }
    out
}

struct Claims {
    claims: BTreeMap<String, Vec<String>>,
    primary: BTreeMap<String, String>,
    paths: BTreeMap<String, std::path::PathBuf>,
}

fn local_claims() -> Result<Claims> {
    let (mut claims, mut primary, mut paths) = (
        BTreeMap::<String, Vec<String>>::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        for extension in local::array(&text, "extensions") {
            claims.entry(extension).or_default().push(id.clone());
        }
        for extension in local::array(&text, "primary-extensions") {
            primary.insert(extension, id.clone());
        }
        paths.insert(id, path);
    }
    Ok(Claims {
        claims,
        primary,
        paths,
    })
}

#[derive(Default)]
struct Verdicts {
    agreed: BTreeMap<String, String>,
    single: BTreeMap<String, String>,
    disputed: BTreeMap<String, (Vec<String>, Vec<String>)>,
    unhelped: Vec<String>,
}

fn classify(
    claims: &Claims,
    tokei: &BTreeMap<String, BTreeSet<String>>,
    scc: &BTreeMap<String, BTreeSet<String>>,
) -> Verdicts {
    let mut out = Verdicts::default();
    for (extension, claimants) in &claims.claims {
        if claimants.len() < 2 || claims.primary.contains_key(extension) {
            continue;
        }
        let narrow = |corpus: &BTreeMap<String, BTreeSet<String>>| -> BTreeSet<String> {
            corpus
                .get(extension)
                .map(|named| {
                    named
                        .iter()
                        .filter(|name| claimants.contains(name))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        };
        let (left, right) = (narrow(tokei), narrow(scc));
        // One claimant, named by both: corroborated, and safe to write.
        if left.len() == 1 && left == right {
            out.agreed.insert(
                extension.clone(),
                left.iter().next().cloned().unwrap_or_default(),
            );
        } else if left.len() == 1 && right.is_empty() {
            out.single.insert(
                extension.clone(),
                left.iter().next().cloned().unwrap_or_default(),
            );
        } else if right.len() == 1 && left.is_empty() {
            out.single.insert(
                extension.clone(),
                right.iter().next().cloned().unwrap_or_default(),
            );
        } else if !left.is_empty() && !right.is_empty() && left != right {
            out.disputed.insert(
                extension.clone(),
                (left.into_iter().collect(), right.into_iter().collect()),
            );
        } else {
            out.unhelped.push(extension.clone());
        }
    }
    out
}

fn object(raw: &str, what: &str) -> Result<serde_json::Map<String, Value>> {
    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("{what} did not parse: {error}"))?;
    let value = value
        .get("languages")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or(value);
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{what} is not an object").into())
}

/// Python renders a sorted list of strings as `['a', 'b']` in these messages.
fn debug_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn run(verb: &str) -> Result<Outcome> {
    let claims = local_claims()?;
    let tokei = owners(&object(&fetch::text(TOKEI)?, "tokei languages.json")?);
    let scc = owners(&object(&fetch::text(SCC)?, "scc languages.json")?);
    let verdicts = classify(&claims, &tokei, &scc);

    if verb == "check" {
        println!(
            "contested extensions resolving to nothing: {}",
            verdicts.agreed.len()
                + verdicts.single.len()
                + verdicts.disputed.len()
                + verdicts.unhelped.len()
        );
        println!(
            "  both corpora agree, and can be applied : {}",
            verdicts.agreed.len()
        );
        println!(
            "  one corpus only, left for a person     : {}",
            verdicts.single.len()
        );
        println!(
            "  the corpora disagree, left alone       : {}",
            verdicts.disputed.len()
        );
        for (extension, (left, right)) in &verdicts.disputed {
            println!(
                "    .{extension}: tokei {} scc {}",
                debug_list(left),
                debug_list(right)
            );
        }
        println!(
            "  no corpus has an opinion               : {}",
            verdicts.unhelped.len()
        );
        return Ok(Outcome::of(verdicts.agreed.len()));
    }

    let mut by_language: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (extension, language) in &verdicts.agreed {
        by_language
            .entry(language.clone())
            .or_default()
            .push(extension.clone());
    }
    for (language, extensions) in &by_language {
        let Some(path) = claims.paths.get(language) else {
            continue;
        };
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut all: BTreeSet<String> = local::array(&text, "primary-extensions")
            .into_iter()
            .collect();
        all.extend(extensions.iter().cloned());
        let all: Vec<String> = all.into_iter().collect();
        let line = format!("primary-extensions = {}\n", local::toml_array(&all));
        let text = match local::span(&text, "primary-extensions") {
            Some((start, end)) => format!("{}{line}{}", &text[..start], &text[end..]),
            None => match local::span(&text, "extensions") {
                Some((_, end)) => format!("{}{line}{}", &text[..end], &text[end..]),
                None => continue,
            },
        };
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }

    let mut rows = String::new();
    for (extension, language) in &verdicts.single {
        let _ = write!(
            rows,
            "\n[[gap]]\nsubject = {}\nreason = \"uncorroborated\"\nnote = {}\n",
            local::toml_string(extension),
            local::toml_string(&format!("one corpus names {language}; the other is silent"))
        );
    }
    for (extension, (left, right)) in &verdicts.disputed {
        let _ = write!(
            rows,
            "\n[[gap]]\nsubject = {}\nreason = \"sources-disagree\"\nnote = {}\n",
            local::toml_string(extension),
            local::toml_string(&format!(
                "tokei says {}, scc says {}",
                debug_list(left),
                debug_list(right)
            ))
        );
    }
    std::fs::write(
        "data/gaps/extension-owner.toml",
        format!(
            "# Contested extensions no corroborated source settles. Detection\n\
             # declines rather than guessing; these say why.\n\
             facet = \"extension-owner\"\n{rows}"
        ),
    )
    .map_err(|e| format!("data/gaps/extension-owner.toml: {e}"))?;

    println!(
        "settled {} contested extensions across {} languages; {} single-source and {} disputed \
         left alone",
        verdicts.agreed.len(),
        by_language.len(),
        verdicts.single.len(),
        verdicts.disputed.len()
    );
    Ok(Outcome::Complete)
}
