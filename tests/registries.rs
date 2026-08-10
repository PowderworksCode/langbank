//! The registries populate, and every fact in them is reachable.
//!
//! These exist because registration goes through `inventory`, which collects
//! at link time. A crate that compiles proves nothing about whether its
//! profiles actually registered — an integration test that links the whole
//! library is the only thing that does. This is the acceptance evidence for
//! the lift out of entl.

use std::collections::BTreeSet;
use std::path::Path;

use langbank::*;

#[test]
fn every_registry_populates() {
    assert!(!language_profiles().is_empty(), "languages");
    assert!(!ecosystem_profiles().is_empty(), "ecosystems");
    assert!(!tool_profiles().is_empty(), "tools");
    assert!(!artifact_profiles().is_empty(), "artifacts");
    assert!(!language_facets().is_empty(), "facets");
    assert!(!traversal_directories().is_empty(), "traversal");
    assert!(verbosity_ratios().next().is_some(), "verbosity");
}

#[test]
fn language_ids_are_unique() {
    let ids = language_profiles()
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ids.len(),
        language_profiles().len(),
        "two profiles share an id"
    );
}

#[test]
fn every_curated_extension_resolves() {
    // Only curated ones. An imported extension claimed by several languages
    // deliberately resolves to nothing — see `imported_languages.rs`.
    for profile in language_profiles()
        .iter()
        .filter(|profile| profile.provenance.is_curated())
    {
        for extension in profile.extensions {
            assert!(
                language_profile_for_extension(extension).is_some(),
                "{extension} is declared by {} and resolves to nothing",
                profile.id
            );
        }
    }
}

#[test]
fn detection_reads_extensions_filenames_and_shebangs() {
    let by_extension = detect_language(Path::new("src/main.rs"), None).expect("rust by extension");
    assert_eq!(by_extension.language.as_str(), "rust");
    assert!(matches!(
        by_extension.evidence.first(),
        Some(LanguageEvidence::Extension { .. })
    ));

    let by_shebang = detect_language(Path::new("scripts/deploy"), Some(b"#!/usr/bin/env bash\n"))
        .expect("shell by shebang");
    assert_eq!(by_shebang.language.as_str(), "shell");
    assert!(matches!(
        by_shebang.evidence.first(),
        Some(LanguageEvidence::Shebang { .. })
    ));

    assert!(detect_language(Path::new("notes.unknownext"), None).is_none());
}

#[test]
fn profiles_carry_the_facts_consumers_ask_for() {
    let rust = language_profile("rust").expect("rust registered");
    assert_eq!(rust.role, LanguageRole::Programming);
    assert!(rust.detects_source(Path::new("lib.rs")));
    assert!(comment_syntax("rust").is_some_and(|syntax| syntax.line.contains(&"//")));
    assert!(comment_syntax_for_extension("rs").is_some());
    assert!(
        rust.conventions.is_some(),
        "rust declares test-layout conventions"
    );
}

#[test]
fn verbosity_is_a_measured_pair_relation() {
    assert!(verbosity("rust").is_some());
    assert!(verbosity_ratio("rust", "typescript").is_some());
    assert!(verbosity_ratio("rust", "no-such-language").is_none());
}

#[test]
fn a_pin_policy_reads_a_source_and_a_requirement() {
    // The one signature changed during the lift. It takes the taxonomy and the
    // spec, never the parsed manifest record, which belongs to whoever read the
    // manifest — that inversion is what lets this crate sit below entl.
    let cargo = ecosystem_profile("cargo").expect("cargo registered");
    let policy = cargo.dependency_pins.expect("cargo declares a pin policy");

    assert_eq!(
        policy.classify(DependencySource::LocalPath, None),
        DependencyPinStatus::Local
    );
    assert_eq!(
        policy.classify(DependencySource::Workspace, None),
        DependencyPinStatus::Local
    );
    assert_eq!(
        policy.classify(DependencySource::Registry, Some("=1.2.3")),
        DependencyPinStatus::Pinned
    );
    assert_eq!(
        policy.classify(DependencySource::Registry, Some("^1.2")),
        DependencyPinStatus::Floating
    );
    // a git dependency is pinned only by a commit sha, not by a branch
    assert_eq!(
        policy.classify(DependencySource::Git, Some("main")),
        DependencyPinStatus::Floating
    );
}

#[test]
fn a_filename_identifies_a_language_with_no_extension() {
    let dockerfile = detect_language(Path::new("Dockerfile"), None).expect("by filename");
    assert_eq!(dockerfile.language.as_str(), "dockerfile");
    assert!(matches!(
        dockerfile.evidence.first(),
        Some(LanguageEvidence::Filename { .. })
    ));
}

#[test]
fn identifiers_round_trip_as_bare_strings() {
    let id = LanguageId::new("rust");
    let encoded = serde_json::to_string(&id).expect("encode");
    assert_eq!(encoded, "\"rust\"");
    assert_eq!(
        serde_json::from_str::<LanguageId>(&encoded).expect("decode"),
        id
    );

    let detection = LanguageDetection {
        language: LanguageId::new("rust"),
        evidence: vec![LanguageEvidence::Extension {
            extension: "rs".to_owned(),
        }],
    };
    let encoded = serde_json::to_string(&detection).expect("encode");
    assert_eq!(
        serde_json::from_str::<LanguageDetection>(&encoded).expect("decode"),
        detection
    );
}
