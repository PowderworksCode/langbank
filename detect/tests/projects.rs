//! The project-recognition data, actually run.
//!
//! Manifests, lockfiles, selector files and `manifest-selection` have been in
//! `data/ecosystems/` since the beginning and nothing has ever executed them.

use langbank_detect::project::{Evidence, claims, identify_project};
use std::collections::BTreeSet;

fn listing(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn a_lockfile_settles_what_a_manifest_cannot() {
    // Four package managers read package.json. The lockfile is the difference.
    let shared = claims(&listing(&["package.json"]));
    assert!(
        shared.len() > 1,
        "package.json should be contested, got {:?}",
        shared.iter().map(|c| c.ecosystem.id).collect::<Vec<_>>()
    );

    let bun = identify_project(&listing(&["package.json", "bun.lock"])).expect("bun");
    assert_eq!(bun.ecosystem.id, "bun");
    assert!(bun.is_decisive());
    assert!(
        bun.evidence
            .iter()
            .any(|found| matches!(found, Evidence::Lockfile(name) if name == "bun.lock")),
        "{:?}",
        bun.evidence
    );
}

#[test]
fn a_shared_manifest_falls_back_to_the_one_declared_default() {
    // Exactly one ecosystem per manifest is the default — the registry asserts
    // it at build time — so a bare package.json is npm rather than a coin toss
    // between four. This is the convention the data states, not a guess.
    let npm = identify_project(&listing(&["package.json"])).expect("npm by default");
    assert_eq!(npm.ecosystem.id, "npm");
    assert!(
        !npm.is_decisive(),
        "a shared manifest is not decisive evidence"
    );

    let poetry = identify_project(&listing(&["pyproject.toml"])).expect("poetry by default");
    assert_eq!(poetry.ecosystem.id, "poetry");
}

#[test]
fn two_lockfiles_are_reported_rather_than_resolved() {
    // A repository mid-migration from yarn to pnpm has both, which is a real
    // state. Saying "I cannot tell" is more use than naming one of them.
    let both = listing(&["package.json", "yarn.lock", "pnpm-lock.yaml"]);
    assert!(identify_project(&both).is_none());
    let found = claims(&both);
    let decisive: Vec<&str> = found
        .iter()
        .filter(|claim| claim.is_decisive())
        .map(|claim| claim.ecosystem.id)
        .collect();
    assert_eq!(decisive, ["pnpm", "yarn"], "both should still be reported");
}

#[test]
fn an_unshared_manifest_is_enough_on_its_own() {
    // Nothing else reads Cargo.toml, so no lockfile is needed to be sure.
    let cargo = identify_project(&listing(&["Cargo.toml"])).expect("cargo");
    assert_eq!(cargo.ecosystem.id, "cargo");
}

#[test]
fn an_empty_directory_claims_nothing() {
    assert!(claims(&listing(&[])).is_empty());
    assert!(identify_project(&listing(&["README.md", "src"])).is_none());
}

#[test]
fn every_ecosystem_can_be_recognised_by_its_own_data() {
    // If an ecosystem's manifest and lockfiles cannot produce a claim on a
    // directory containing exactly those files, its entry describes something
    // no walker could ever find.
    for ecosystem in langbank::ecosystem_profiles() {
        let mut names: Vec<&str> = ecosystem.lockfiles.to_vec();
        names.extend(ecosystem.selector_files);
        if let Some(manifest) = ecosystem.manifest {
            names.push(manifest);
        }
        if names.is_empty() {
            continue;
        }
        let found = claims(&listing(&names));
        assert!(
            found.iter().any(|claim| claim.ecosystem.id == ecosystem.id),
            "{} cannot be recognised from {names:?}",
            ecosystem.id
        );
    }
}

#[test]
fn exactly_one_ecosystem_per_manifest_is_the_default() {
    // The registry asserts this when it builds, so this test is really about
    // saying what the rule is: my first version of it asserted the opposite —
    // that every sharer must be `Lockfile` — and would have "fixed" correct
    // data into a manifest nobody could resolve.
    let mut by_manifest: std::collections::BTreeMap<&str, Vec<&str>> = Default::default();
    for ecosystem in langbank::ecosystem_profiles() {
        if let Some(manifest) = ecosystem.manifest {
            by_manifest.entry(manifest).or_default().push(ecosystem.id);
        }
    }
    for (manifest, sharers) in &by_manifest {
        let defaults: Vec<&&str> = sharers
            .iter()
            .filter(|id| {
                langbank::ecosystem_profile(id)
                    .is_some_and(|e| e.manifest_selection == langbank::ManifestSelection::Default)
            })
            .collect();
        assert_eq!(defaults.len(), 1, "{manifest} has defaults {defaults:?}");
        // And that default is exactly who a bare manifest resolves to.
        let found = identify_project(&listing(&[manifest])).expect(manifest);
        assert_eq!(found.ecosystem.id, *defaults[0], "{manifest}");
    }
}
