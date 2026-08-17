//! Which ecosystems claim a directory, and on what evidence.
//!
//! langbank describes how to recognise a project — a manifest, the lockfiles
//! that pin it, selector files, the alternates it also accepts — and until now
//! nothing ran any of it. That is the state the content rules were in before
//! `identify`, which is where the `^`-anchoring bug was hiding.
//!
//! The filesystem stays out here. This takes the names in a directory as a set,
//! so the leaf keeps its promise that nothing in it walks a filesystem, and a
//! caller that already has a listing does not pay for a second one.

use langbank::{EcosystemProfile, ManifestSelection};
use std::collections::BTreeSet;

/// Why an ecosystem was matched. Ordered by how much it settles: a lockfile
/// names the tool exactly, a manifest only names the format.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    /// A lockfile only this ecosystem writes. `bun.lock` is Bun and nothing
    /// else, where `package.json` is four package managers.
    Lockfile(String),
    /// A file that selects this ecosystem among several sharing a manifest.
    Selector(String),
    /// The manifest itself.
    Manifest(String),
    /// A manifest it also accepts, rather than its primary one.
    Alternate(String),
}

impl Evidence {
    /// Whether this evidence alone identifies the ecosystem, as opposed to
    /// narrowing it. `manifest-selection = "lockfile"` is the data saying its
    /// manifest is shared and cannot decide on its own.
    pub fn is_decisive(&self) -> bool {
        matches!(self, Evidence::Lockfile(_) | Evidence::Selector(_))
    }
}

/// One ecosystem's claim on a directory.
#[derive(Debug, Clone)]
pub struct Claim {
    pub ecosystem: &'static EcosystemProfile,
    /// Everything found, strongest first.
    pub evidence: Vec<Evidence>,
}

impl Claim {
    pub fn is_decisive(&self) -> bool {
        self.evidence.iter().any(Evidence::is_decisive)
    }
}

/// What the directory listing says, by itself.
fn claim(ecosystem: &'static EcosystemProfile, files: &BTreeSet<String>) -> Option<Claim> {
    let mut evidence = Vec::new();
    for lockfile in ecosystem.lockfiles {
        if files.contains(*lockfile) {
            evidence.push(Evidence::Lockfile((*lockfile).to_string()));
        }
    }
    for selector in ecosystem.selector_files {
        if files.contains(*selector) {
            evidence.push(Evidence::Selector((*selector).to_string()));
        }
    }
    if let Some(manifest) = ecosystem.manifest
        && files.contains(manifest)
    {
        evidence.push(Evidence::Manifest(manifest.to_string()));
    }
    for alternate in ecosystem.alternate_manifests {
        if files.contains(*alternate) {
            evidence.push(Evidence::Alternate((*alternate).to_string()));
        }
    }
    evidence.sort();
    (!evidence.is_empty()).then_some(Claim {
        ecosystem,
        evidence,
    })
}

/// Every ecosystem with any claim on this directory, strongest first.
///
/// More than one is the normal case and not a failure: a JavaScript project
/// with a `package.json` is claimed by npm, pnpm, yarn and Bun until a lockfile
/// says which. Returning them all, ordered, lets a caller decide how much
/// certainty it needs.
pub fn claims(files: &BTreeSet<String>) -> Vec<Claim> {
    let mut found: Vec<Claim> = langbank::ecosystem_profiles()
        .iter()
        .filter_map(|ecosystem| claim(ecosystem, files))
        .collect();
    found.sort_by(|left, right| {
        right
            .is_decisive()
            .cmp(&left.is_decisive())
            .then_with(|| left.ecosystem.id.cmp(right.ecosystem.id))
    });
    found
}

/// The single ecosystem this directory belongs to, when the evidence settles it.
///
/// `None` where several claim it and none decisively — which is a real answer
/// about a `package.json` with no lockfile, not a failure to look properly.
pub fn identify_project(files: &BTreeSet<String>) -> Option<Claim> {
    let found = claims(files);
    let decisive: Vec<&Claim> = found.iter().filter(|claim| claim.is_decisive()).collect();
    if let [only] = decisive.as_slice() {
        return Some((*only).clone());
    }
    if !decisive.is_empty() {
        // Two lockfiles in one directory is a real repository state and not
        // something to guess at — a project mid-migration from yarn to pnpm has
        // both, and saying so is more use than picking one.
        return None;
    }
    // Nothing decisive. The manifest may still settle it, because exactly one
    // ecosystem per manifest is declared the default: `package.json` with no
    // lockfile is npm, which is the convention the data states rather than a
    // coin toss between four.
    let defaulting: Vec<&Claim> = found
        .iter()
        .filter(|claim| {
            claim.ecosystem.manifest_selection == ManifestSelection::Default
                && claim
                    .evidence
                    .iter()
                    .any(|found| matches!(found, Evidence::Manifest(_) | Evidence::Alternate(_)))
        })
        .collect();
    match defaulting.as_slice() {
        [only] => Some((*only).clone()),
        _ => None,
    }
}
