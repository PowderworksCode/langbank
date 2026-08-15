//! Rules that name a language by reading a file.
//!
//! Langbank declines to name a language when several claim an extension and
//! none claims to win. That is honest and unhelpful, and it is only unavoidable
//! because langbank does not read files.
//!
//! So it carries the rules instead of the answers, exactly as it carries a
//! version probe rather than a version: a consumer that has the bytes runs
//! them, and one that does not is no worse off than before. The patterns are
//! strings for the same reason the probes are — compiling a regex is the
//! consumer's job, and a regex crate is not a dependency this leaf imposes.
//!
//! Rules are ordered and the first whose clauses all hold wins. A rule with no
//! clauses always holds, which is how a fallback is spelled.

use std::sync::LazyLock;

use crate::{LanguageProfile, registry};

/// One condition. Any pattern may match; no negative pattern may.
#[derive(Debug, Clone, Copy)]
pub struct Clause {
    pub patterns: &'static [&'static str],
    pub negative: &'static [&'static str],
}

/// A language, and what must hold of the content for it to be the answer.
#[derive(Debug, Clone, Copy)]
pub struct DisambiguationRule {
    pub language: &'static LanguageProfile,
    /// All must hold. Empty means the rule always holds — linguist's fallback.
    pub clauses: &'static [Clause],
    /// Whether the patterns avoid lookaround and backreferences. Three rules of
    /// 317 do not, and a consumer using Rust's regex crate cannot compile those
    /// — which is better said here than discovered at run time.
    pub portable: bool,
}

/// The ordered rules for one group of extensions.
#[derive(Debug, Clone, Copy)]
pub struct Disambiguation {
    pub extensions: &'static [&'static str],
    pub rules: &'static [DisambiguationRule],
}

#[derive(Debug, Clone, Copy)]
pub struct DisambiguationRegistration(pub &'static Disambiguation);

registry::collect!(DisambiguationRegistration);

static REGISTERED: LazyLock<Vec<&'static Disambiguation>> = LazyLock::new(|| {
    let mut blocks = registry::iter::<DisambiguationRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    blocks.sort_by_key(|block| block.extensions.first().copied().unwrap_or_default());
    blocks
});

pub fn disambiguations() -> &'static [&'static Disambiguation] {
    &REGISTERED
}

/// How to decide what an ambiguous extension is, by reading the file.
///
/// Returns nothing where langbank already answers outright, and nothing where
/// no rules exist — [`crate::gap`] says which of those it is.
pub fn disambiguation_for(extension: &str) -> Option<&'static Disambiguation> {
    let extension = extension.to_ascii_lowercase();
    disambiguations()
        .iter()
        .copied()
        .find(|block| block.extensions.contains(&extension.as_str()))
}
