use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use crate::{LanguageDetection, LanguageEvidence, LanguageId};

use crate::{
    LanguageConventions, LanguageFacet, LanguageVerbosity, language_facets, registry, verbosity,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageRole {
    Programming,
    Markup,
    Stylesheet,
    Data,
    Documentation,
    Build,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentSyntax {
    pub line: &'static [&'static str],
    pub block: &'static [(&'static str, &'static str)],
    /// Prefixes recognized by the language's documentation tooling.
    pub documentation: &'static [&'static str],
    pub quotes: &'static [char],
    pub multi_quotes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    /// Extensions that directly identify files of this language.
    pub extensions: &'static [&'static str],
    /// Extensions accepted when a consumer treats this language as a project overlay.
    pub source_extensions: &'static [&'static str],
    pub filenames: &'static [&'static str],
    pub shebangs: &'static [&'static str],
    pub role: LanguageRole,
    pub facets: &'static [&'static LanguageFacet],
    pub comments: Option<&'static CommentSyntax>,
    pub conventions: Option<LanguageConventions>,
    pub config_files: &'static [&'static str],
    pub package_dependencies: &'static [&'static str],
    pub supersedes: &'static [&'static LanguageProfile],
    /// The language this one is counted under, as linguist groups it.
    ///
    /// BibTeX groups under TeX, an APKBUILD under Shell, Bison under Yacc.
    /// This is linguist's statistical rollup and not a claim about language
    /// design: it says a consumer totalling bytes by language may want to add
    /// this one to that one, which is the question linguist invented it to
    /// answer.
    ///
    /// Distinct from `supersedes`, which says one language replaced another.
    /// TypeScript supersedes JavaScript and is grouped under nothing.
    pub groups_under: Option<&'static LanguageProfile>,
    /// Tokens this language wins when several claim them. Detection declines
    /// to answer a contested token unless exactly one claimant says this.
    pub primary_extensions: &'static [&'static str],
}

impl LanguageProfile {
    pub fn detects_source(&self, path: &Path) -> bool {
        extension(path).is_some_and(|extension| self.extensions.contains(&extension.as_str()))
    }

    pub fn accepts_source(&self, path: &Path) -> bool {
        extension(path)
            .is_some_and(|extension| self.source_extensions.contains(&extension.as_str()))
    }

    /// Every language counted under this one, directly.
    pub fn dialects(&self) -> Vec<&'static LanguageProfile> {
        crate::language_profiles()
            .iter()
            .filter(|other| {
                other
                    .groups_under
                    .is_some_and(|parent| std::ptr::eq(parent, self))
            })
            .copied()
            .collect()
    }

    pub fn supersedes(&self, other: &LanguageProfile) -> bool {
        self.supersedes
            .iter()
            .any(|profile| std::ptr::eq(*profile, other))
    }

    /// How much source text this language needs relative to
    /// [`VERBOSITY_BASELINE`](super::VERBOSITY_BASELINE), on the corpus named
    /// by [`VERBOSITY_CORPUS`](super::VERBOSITY_CORPUS). Languages with no
    /// algorithmic presence there, such as CSS or YAML, have no measurement.
    ///
    /// The number is as much a property of that corpus as of the language.
    /// Measured on a mid-sized program instead of small exercises, the same
    /// languages spread about twice as far apart; see `notes/verbosity.md`.
    pub fn verbosity(&self) -> Option<&'static LanguageVerbosity> {
        verbosity(self.id)
    }

    pub fn has_facet(&self, facet: &LanguageFacet) -> bool {
        self.facets
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, facet))
    }

    pub fn detects_project(
        &self,
        files: &BTreeSet<String>,
        dependencies: &BTreeSet<String>,
    ) -> bool {
        self.config_files.iter().any(|file| files.contains(*file))
            || self
                .package_dependencies
                .iter()
                .any(|dependency| dependencies.contains(*dependency))
    }
}

impl From<&LanguageProfile> for LanguageId {
    fn from(profile: &LanguageProfile) -> Self {
        Self::new(profile.id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageRegistration(pub &'static LanguageProfile);

registry::collect!(LanguageRegistration);

static REGISTERED: LazyLock<Vec<&'static LanguageProfile>> = LazyLock::new(|| {
    let mut profiles = registry::iter::<LanguageRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.id);
    for pair in profiles.windows(2) {
        assert_ne!(pair[0].id, pair[1].id, "duplicate language profile ID");
    }
    let mut extensions = BTreeSet::new();
    for profile in &profiles {
        let mut profile_facets = BTreeSet::new();
        for facet in profile.facets {
            assert!(
                language_facets()
                    .iter()
                    .any(|registered| std::ptr::eq(*registered, *facet)),
                "language profile {:?} references unregistered facet {:?}",
                profile.id,
                facet.id
            );
            assert!(
                profile_facets.insert(facet.id),
                "language profile {:?} repeats facet {:?}",
                profile.id,
                facet.id
            );
        }
        // Extensions are contested constantly and legitimately once a
        // language registry is complete: `.inc` belongs to twelve languages and
        // `.h` to three. What must stay unique is a *claim to win* one, since
        // two languages both declaring `.rs` primary is somebody's mistake.
        for extension in profile.primary_extensions {
            assert!(
                extensions.insert(*extension),
                "two languages both claim {extension:?} as primary"
            );
        }
        for superseded in profile.supersedes {
            assert!(
                profiles
                    .iter()
                    .any(|registered| std::ptr::eq(*registered, *superseded)),
                "language profile {:?} supersedes unregistered profile {:?}",
                profile.id,
                superseded.id
            );
        }
    }
    profiles
});

pub fn language_profiles() -> &'static [&'static LanguageProfile] {
    REGISTERED.as_slice()
}

pub fn language_profile(id: &str) -> Option<&'static LanguageProfile> {
    language_profiles()
        .binary_search_by_key(&id, |profile| profile.id)
        .ok()
        .map(|index| language_profiles()[index])
}

impl LanguageDetection {
    pub fn profile(&self) -> Option<&'static LanguageProfile> {
        language_profile(self.language.as_str())
    }
}

/// Which language a token identifies, when exactly one language does.
///
/// Most tokens have a single claimant and resolve outright. 176 extensions are
/// contested — `.inc` is claimed by twelve languages and `.h` by three — and a
/// contest is settled only when exactly one claimant declares the token
/// primary. Otherwise detection returns nothing, because choosing without
/// reading the file is a wrong answer rather than a missing one, and
/// `languages_claiming_extension` hands a consumer the candidates instead.
fn index(
    claims: fn(&'static LanguageProfile) -> &'static [&'static str],
    primary: fn(&'static LanguageProfile) -> &'static [&'static str],
) -> BTreeMap<&'static str, &'static LanguageProfile> {
    let mut claimants: BTreeMap<&str, Vec<&'static LanguageProfile>> = BTreeMap::new();
    for profile in language_profiles() {
        for token in claims(profile) {
            claimants.entry(token).or_default().push(profile);
        }
    }
    claimants
        .into_iter()
        .filter_map(|(token, profiles)| {
            let resolved = match profiles.as_slice() {
                [only] => Some(*only),
                contested => {
                    let mut preferring = contested
                        .iter()
                        .filter(|profile| primary(profile).contains(&token));
                    match (preferring.next(), preferring.next()) {
                        (Some(winner), None) => Some(*winner),
                        _ => None,
                    }
                }
            };
            resolved.map(|profile| (token, profile))
        })
        .collect()
}

static BY_EXTENSION: LazyLock<BTreeMap<&'static str, &'static LanguageProfile>> =
    LazyLock::new(|| {
        index(
            |profile| profile.extensions,
            |profile| profile.primary_extensions,
        )
    });

static BY_FILENAME: LazyLock<BTreeMap<&'static str, &'static LanguageProfile>> =
    LazyLock::new(|| index(|profile| profile.filenames, |_| &[]));

pub fn language_profile_for_extension(extension: &str) -> Option<&'static LanguageProfile> {
    let extension = extension.to_ascii_lowercase();
    BY_EXTENSION.get(extension.as_str()).copied()
}

/// Every language that claims this extension, contested or not.
pub fn languages_claiming_extension(extension: &str) -> Vec<&'static LanguageProfile> {
    let extension = extension.to_ascii_lowercase();
    language_profiles()
        .iter()
        .copied()
        .filter(|profile| profile.extensions.contains(&extension.as_str()))
        .collect()
}

pub fn comment_syntax(language: &str) -> Option<&'static CommentSyntax> {
    language_profile(language).and_then(|profile| profile.comments)
}

pub fn comment_syntax_for_extension(extension: &str) -> Option<&'static CommentSyntax> {
    language_profile_for_extension(extension).and_then(|profile| profile.comments)
}

pub fn detect_language(path: &Path, prefix: Option<&[u8]>) -> Option<LanguageDetection> {
    let filename = path.file_name()?.to_str()?;
    if let Some(profile) = BY_FILENAME.get(filename).copied() {
        return Some(LanguageDetection {
            language: LanguageId::from(profile),
            evidence: vec![LanguageEvidence::Filename {
                filename: filename.to_owned(),
            }],
        });
    }

    if let Some(extension) = extension(path)
        && let Some(profile) = language_profile_for_extension(&extension)
    {
        return Some(LanguageDetection {
            language: LanguageId::from(profile),
            evidence: vec![LanguageEvidence::Extension { extension }],
        });
    }

    let prefix = prefix?;
    let first_line = prefix.split(|byte| *byte == b'\n').next()?;
    let shebang = std::str::from_utf8(first_line).ok()?.strip_prefix("#!")?; // straitjacket-allow:error-discard — a file whose first line is not UTF-8 has no shebang
    let normalized = shebang.to_ascii_lowercase();
    let profile = language_profiles().iter().copied().find(|profile| {
        profile
            .shebangs
            .iter()
            .any(|needle| contains_interpreter(&normalized, needle))
    })?;
    Some(LanguageDetection {
        language: LanguageId::from(profile),
        evidence: vec![LanguageEvidence::Shebang {
            interpreter: shebang.trim().to_owned(),
        }],
    })
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

fn contains_interpreter(shebang: &str, interpreter: &str) -> bool {
    shebang
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        .filter(|part| !part.is_empty())
        .any(|part| {
            part == interpreter
                || part.strip_prefix(interpreter).is_some_and(|suffix| {
                    suffix.chars().all(|character| character.is_ascii_digit())
                })
        })
}
