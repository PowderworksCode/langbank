//! The pruning and artifact data, run.

use langbank_detect::walk::{artifacts, prunable, prune};
use std::collections::BTreeSet;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn a_conditional_prune_needs_its_marker() {
    // `target` is Cargo's only when Cargo.toml is beside it. Skipping it
    // anywhere else would silently lose somebody's source directory, which is
    // the failure this rule exists to prevent.
    let rust = set(&["Cargo.toml", "src", "target"]);
    let pruned = prune("target", &rust).expect("target beside Cargo.toml");
    assert_eq!(pruned.because, Some("Cargo.toml"));

    let not_rust = set(&["Makefile", "src", "target"]);
    assert!(
        prune("target", &not_rust).is_none(),
        "target without Cargo.toml is not Cargo's and must be walked"
    );
}

#[test]
fn an_unconditional_prune_needs_nothing() {
    // node_modules is generated wherever it appears, so it carries no markers.
    let pruned = prune("node_modules", &set(&["node_modules"])).expect("node_modules");
    assert_eq!(pruned.because, None);
    assert!(pruned.directory.markers.is_empty());
}

#[test]
fn a_listing_is_pruned_all_at_once() {
    let project = set(&[
        "package.json",
        "node_modules",
        "dist",
        "coverage",
        "src",
        "README.md",
    ]);
    let skipped: Vec<&str> = prunable(&project)
        .iter()
        .map(|pruned| pruned.directory.name)
        .collect();
    assert_eq!(skipped, ["coverage", "dist", "node_modules"]);
    // And nothing that is not generated.
    assert!(!skipped.contains(&"src"));
}

#[test]
fn every_registered_directory_can_actually_be_pruned() {
    // A rule whose markers never co-occur with its own name describes a
    // directory no walker would ever skip.
    for directory in langbank::traversal_directories() {
        let mut listing = set(&[directory.name]);
        listing.extend(directory.markers.iter().map(|m| (*m).to_string()));
        assert!(
            prune(directory.name, &listing).is_some(),
            "{} cannot be pruned even beside {:?}",
            directory.name,
            directory.markers
        );
    }
}

#[test]
fn an_artifact_is_recognised_by_any_of_its_signals() {
    let none = artifacts(&set(&[]), &[], &set(&[]));
    assert!(none.is_empty(), "{none:?}");

    let by_dependency = artifacts(&set(&["@napi-rs/cli"]), &[], &set(&[]));
    assert_eq!(
        by_dependency.iter().map(|a| a.id).collect::<Vec<_>>(),
        ["napi"]
    );

    let by_script = artifacts(
        &set(&[]),
        &["bun build --compile ./cli.ts".into()],
        &set(&[]),
    );
    assert_eq!(
        by_script.iter().map(|a| a.id).collect::<Vec<_>>(),
        ["binary"]
    );

    let by_facet = artifacts(&set(&[]), &[], &set(&["tauri"]));
    assert_eq!(by_facet.iter().map(|a| a.id).collect::<Vec<_>>(), ["tauri"]);
}

#[test]
fn every_artifact_profile_can_be_reached_by_something() {
    // A profile with no dependency, no script signal and no facet is
    // unreachable: it describes an artifact nothing could ever detect.
    for profile in langbank::artifact_profiles() {
        assert!(
            !profile.package_dependencies.is_empty()
                || !profile.package_script_signals.is_empty()
                || !profile.project_facets.is_empty(),
            "{} has no signal and can never be found",
            profile.id
        );
    }
}
