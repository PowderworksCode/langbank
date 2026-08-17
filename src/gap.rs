//! What langbank knows it does not know.
//!
//! Absorbing seven sources turns up disagreements, and until now they were
//! printed and thrown away — 551 findings rediscovered on every sync run and
//! discarded again. That was tolerable with one source and is not with seven,
//! because the interesting part of merging is precisely where sources differ.
//!
//! A gap is an absence with a reason. It lets a consumer tell three things
//! apart that all look identical from the outside: a fact nobody has recorded,
//! a fact two sources contradict each other about, and a fact one source
//! asserts that nothing has confirmed. Langbank still declines to answer in all
//! three cases — the difference is that it can now say why.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use crate::registry;

/// Why langbank has no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapReason {
    /// Two independent sources contradict each other, so neither is taken.
    /// `.luau` is Lua to tokei and Luau to scc.
    SourcesDisagree,
    /// One source asserts it and nothing corroborates. Enough to record, not
    /// enough to act on where acting means overruling other claimants.
    Uncorroborated,
    /// Upstream has it and langbank deliberately did not take it, which is a
    /// decision rather than an oversight and is recorded so it reads as one.
    Excluded,
    /// Nobody has modelled it. The ordinary state of most of the registry.
    NotModelled,
    /// Looked for and not there. CSV has no compiler and JSON has no package
    /// manager, and those are answers rather than holes.
    ///
    /// Distinct from `NotModelled`, which means nobody has checked. Recording
    /// the difference is the point: a reader who wants to fill a gap needs to
    /// know which ones are worth opening, and a consumer asking "does this
    /// language have an ecosystem" deserves "no" rather than silence.
    NotApplicable,
}

/// One thing langbank cannot answer, and why.
#[derive(Debug, Clone, Copy)]
pub struct Gap {
    /// What the gap is about: a language id, an extension, a tool name.
    pub subject: &'static str,
    /// Which kind of fact is missing — `comment-syntax`, `extension-owner`.
    pub facet: &'static str,
    pub reason: GapReason,
    /// What the sources actually said, for a person deciding how to close it.
    pub note: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct GapRegistration(pub &'static Gap);

registry::collect!(GapRegistration);

static REGISTERED: LazyLock<Vec<&'static Gap>> = LazyLock::new(|| {
    let mut gaps = registry::iter::<GapRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    gaps.sort_by_key(|gap| (gap.facet, gap.subject));
    let mut seen = BTreeSet::new();
    for gap in &gaps {
        assert!(
            seen.insert((gap.facet, gap.subject)),
            "duplicate gap for {} on {}",
            gap.subject,
            gap.facet
        );
    }
    gaps
});

pub fn gaps() -> &'static [&'static Gap] {
    &REGISTERED
}

/// Everything langbank cannot answer about one subject.
pub fn gaps_for(subject: &str) -> Vec<&'static Gap> {
    gaps()
        .iter()
        .copied()
        .filter(|gap| gap.subject == subject)
        .collect()
}

/// Why langbank has no answer for one fact about one subject, if it has none.
pub fn gap(subject: &str, facet: &str) -> Option<&'static Gap> {
    gaps()
        .iter()
        .copied()
        .find(|entry| entry.subject == subject && entry.facet == facet)
}
