//! Which directories to skip, and what a project builds.
//!
//! The last of langbank's data that nothing executed. Both registries describe
//! decisions a codebase walker has to make on every directory it opens, and
//! neither had a caller.
//!
//! The interesting rule is that pruning is conditional. `node_modules` is
//! always generated, so it carries no markers and is always skipped. `target`
//! is only Cargo's when `Cargo.toml` is beside it — a `target/` directory in a
//! project that is not Rust may be somebody's source, and skipping it would
//! lose real files silently. `build` and `dist` are the same story against
//! `package.json`.

use langbank::{ArtifactProfile, TraversalDirectory};
use std::collections::BTreeSet;

/// Why a directory was skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pruned {
    pub directory: &'static TraversalDirectory,
    /// The marker beside it that confirmed the directory is generated, or
    /// `None` for one that is generated wherever it appears.
    pub because: Option<&'static str>,
}

/// Whether to skip `name`, given what else is in the same directory.
///
/// `siblings` is the listing `name` appears in, not the contents of `name`
/// itself: the evidence that `target` is Cargo's is a `Cargo.toml` next to it.
pub fn prune(name: &str, siblings: &BTreeSet<String>) -> Option<Pruned> {
    for directory in langbank::traversal_directories() {
        if directory.name != name {
            continue;
        }
        if directory.markers.is_empty() {
            return Some(Pruned {
                directory,
                because: None,
            });
        }
        if let Some(marker) = directory
            .markers
            .iter()
            .find(|marker| siblings.contains(**marker))
        {
            return Some(Pruned {
                directory,
                because: Some(marker),
            });
        }
    }
    None
}

/// Every directory in a listing that should not be walked into.
pub fn prunable(listing: &BTreeSet<String>) -> Vec<Pruned> {
    let mut found: Vec<Pruned> = listing
        .iter()
        .filter_map(|name| prune(name, listing))
        .collect();
    found.sort_by_key(|pruned| pruned.directory.name);
    found
}

/// What a project builds, from what its manifest declares.
///
/// Takes the dependency names, the script bodies and the project facets rather
/// than a manifest, because reading `package.json` is the caller's job and
/// three ecosystems spell the same facts differently.
pub fn artifacts(
    dependencies: &BTreeSet<String>,
    scripts: &[String],
    facets: &BTreeSet<String>,
) -> Vec<&'static ArtifactProfile> {
    langbank::artifact_profiles()
        .iter()
        .filter(|profile| {
            profile
                .package_dependencies
                .iter()
                .any(|name| dependencies.contains(*name))
                || profile
                    .package_script_signals
                    .iter()
                    .any(|signal| scripts.iter().any(|script| script.contains(signal)))
                || profile
                    .project_facets
                    .iter()
                    .any(|facet| facets.contains(*facet))
        })
        .copied()
        .collect()
}
