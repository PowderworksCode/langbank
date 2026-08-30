//! What langbank knows about a language, and what it does not.
//!
//! This lives in the leaf rather than in a tool because two things ask it —
//! `langbank-sync coverage` and the facet coverage the documentation site
//! renders from the exported manifest — and a registry whose own coverage
//! report disagrees with its website has a worse problem than a thin facet.
//!
//! It answers only from what is carried. A language with no comment syntax
//! might have none anybody has recorded, or none at all; the distinction is a
//! `Gap`, and this reports the absence rather than explaining it.

use crate::{LanguageProfile, ToolchainKind, ecosystem_profiles, toolchains_for};

/// One thing langbank might know about a language.
///
/// The order is the order they are reported in, which is roughly the order they
/// get filled in: a language is detected before anyone writes down its comment
/// syntax, and its comment syntax before anyone maps its ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Facet {
    /// Enough to recognise a file as this language at all.
    Detection,
    Comments,
    /// Which of the cross-cutting facets — component host, style host — apply.
    Facets,
    /// Test layout, inline-test rules, the things a walker needs.
    Conventions,
    /// Any toolchain at all names this language.
    Toolchain,
    /// Something that builds or runs it, as opposed to checking it.
    Compiler,
    /// A linter or a formatter.
    Analyser,
    /// A package manager that publishes for it.
    Ecosystem,
}

impl Facet {
    pub const ALL: [Facet; 8] = [
        Facet::Detection,
        Facet::Comments,
        Facet::Facets,
        Facet::Conventions,
        Facet::Toolchain,
        Facet::Compiler,
        Facet::Analyser,
        Facet::Ecosystem,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Facet::Detection => "detection",
            Facet::Comments => "comments",
            Facet::Facets => "facets",
            Facet::Conventions => "conventions",
            Facet::Toolchain => "toolchain",
            Facet::Compiler => "compiler",
            Facet::Analyser => "analyser",
            Facet::Ecosystem => "ecosystem",
        }
    }

    /// What carrying this facet would let a consumer do — the reason it is
    /// worth having, rather than a restatement of the name.
    pub fn purpose(self) -> &'static str {
        match self {
            Facet::Detection => "recognise a file as this language",
            Facet::Comments => "strip comments without parsing",
            Facet::Facets => "know it hosts components or styles",
            Facet::Conventions => "find its tests and fixtures",
            Facet::Toolchain => "name a tool that handles it",
            Facet::Compiler => "build or run it",
            Facet::Analyser => "lint or format it",
            Facet::Ecosystem => "resolve its packages",
        }
    }

    fn known_for(self, profile: &'static LanguageProfile) -> bool {
        let serves = toolchains_for(profile);
        let any = |kind| serves.iter().any(|entry| entry.is(kind));
        match self {
            Facet::Detection => {
                !profile.extensions.is_empty()
                    || !profile.filenames.is_empty()
                    || !profile.shebangs.is_empty()
            }
            Facet::Comments => profile.comments.is_some(),
            Facet::Facets => !profile.facets.is_empty(),
            Facet::Conventions => profile.conventions.is_some(),
            Facet::Toolchain => !serves.is_empty(),
            Facet::Compiler => any(ToolchainKind::Compiler) || any(ToolchainKind::Runtime),
            Facet::Analyser => any(ToolchainKind::Linter) || any(ToolchainKind::Formatter),
            Facet::Ecosystem => ecosystem_profiles()
                .iter()
                .any(|ecosystem| ecosystem.implies_language(profile)),
        }
    }
}

/// Which facets are carried for one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Knowledge {
    carried: [bool; 8],
}

impl Knowledge {
    pub fn of(profile: &'static LanguageProfile) -> Self {
        let mut carried = [false; 8];
        for (slot, facet) in carried.iter_mut().zip(Facet::ALL) {
            *slot = facet.known_for(profile);
        }
        Self { carried }
    }

    pub fn has(&self, facet: Facet) -> bool {
        self.carried[facet as usize]
    }

    /// How many of the eight are carried. `0` is possible and is not a bug —
    /// five languages are named by a source and nothing else.
    pub fn count(&self) -> usize {
        self.carried.iter().filter(|have| **have).count()
    }

    pub fn facets(&self) -> impl Iterator<Item = (Facet, bool)> + '_ {
        Facet::ALL.into_iter().map(|facet| (facet, self.has(facet)))
    }
}

/// How many languages carry each facet, in `Facet::ALL` order.
pub fn coverage() -> [usize; 8] {
    let mut totals = [0usize; 8];
    for profile in crate::language_profiles() {
        let knowledge = Knowledge::of(profile);
        for (slot, facet) in totals.iter_mut().zip(Facet::ALL) {
            if knowledge.has(facet) {
                *slot += 1;
            }
        }
    }
    totals
}

/// Coverage split by role: `(role, languages with that role, per-facet counts)`.
///
/// The plain total is misleading and was quietly setting the wrong target. It
/// reports `ecosystem: 25 have, 802 lack`, which reads as 802 languages waiting
/// to be filled in — but JSON has no package manager and CSV has no compiler,
/// and no amount of absorbing will give them one. 270 of the 827 are data,
/// markup, documentation, stylesheet or build languages.
///
/// This does not claim a facet is *inapplicable* to a given language, because
/// that would be a judgement dressed as a fact. It shows the denominator
/// instead and lets a reader see that `0 of 181 data languages have a compiler`
/// is a description of data formats rather than a backlog.
pub fn coverage_by_role() -> Vec<(crate::LanguageRole, usize, [usize; 8])> {
    let mut rows: Vec<(crate::LanguageRole, usize, [usize; 8])> = Vec::new();
    for profile in crate::language_profiles() {
        let knowledge = Knowledge::of(profile);
        let row = match rows.iter_mut().find(|(role, _, _)| *role == profile.role) {
            Some(row) => row,
            None => {
                rows.push((profile.role, 0, [0; 8]));
                rows.last_mut()
                    .unwrap_or_else(|| unreachable!("just pushed"))
            }
        };
        row.1 += 1;
        for (slot, facet) in row.2.iter_mut().zip(Facet::ALL) {
            if knowledge.has(facet) {
                *slot += 1;
            }
        }
    }
    rows.sort_by_key(|(role, count, _)| (std::cmp::Reverse(*count), format!("{role:?}")));
    rows
}

/// How many languages know exactly `n` facets, for `n` in `0..=8`.
///
/// The distribution is the interesting number rather than the average: most
/// languages know exactly one thing about themselves, and a mean would hide
/// that behind the few that know six.
pub fn distribution() -> [usize; 9] {
    let mut histogram = [0usize; 9];
    for profile in crate::language_profiles() {
        histogram[Knowledge::of(profile).count()] += 1;
    }
    histogram
}
