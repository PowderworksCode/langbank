//! Package registries, aligned with purl.
//!
//! The point of this data is a distinction langbank did not previously make: a
//! *registry* is where a package identity lives, and a *manager* is the tool
//! that reads the manifest. npm, pnpm, yarn and bun are four managers over one
//! registry, and until now langbank called all four "ecosystems" and had no way
//! to say they name the same packages.

use langbank::*;

#[test]
fn every_purl_type_is_carried() {
    assert_eq!(package_registries().len(), 42, "one per purl type");
    for id in [
        "npm", "cargo", "pypi", "maven", "gem", "golang", "nuget", "deb", "oci",
    ] {
        assert!(package_registry(id).is_some(), "{id}");
    }
}

#[test]
fn a_registry_knows_where_it_lives_and_how_names_work() {
    let npm = package_registry("npm").expect("npm");
    assert_eq!(npm.default_repository, Some("https://registry.npmjs.org/"));
    assert!(npm.uses_repository);
    // an npm scope is the optional namespace, and case matters
    assert_eq!(npm.namespace.requirement, Requirement::Optional);
    assert_eq!(npm.name.requirement, Requirement::Required);
    assert!(npm.name.case_sensitive);

    // `generic` has no canonical host, and saying so is the point of the flag
    let generic = package_registry("generic").expect("generic");
    assert!(!generic.uses_repository || generic.default_repository.is_none());
}

#[test]
fn four_managers_point_at_one_registry() {
    // This is the distinction the whole change exists for.
    let npm = package_registry("npm").expect("npm registry");
    for manager in ["npm", "pnpm", "yarn", "bun"] {
        let ecosystem = ecosystem_profile(manager).unwrap_or_else(|| panic!("{manager}"));
        let pointed = ecosystem
            .registry
            .unwrap_or_else(|| panic!("{manager} names no registry"));
        assert!(
            std::ptr::eq(pointed, npm),
            "{manager} should publish to the npm registry"
        );
    }
    // and they are still distinguishable, by the thing that actually differs
    assert_ne!(
        ecosystem_profile("pnpm").expect("pnpm").lockfiles,
        ecosystem_profile("yarn").expect("yarn").lockfiles
    );
}

#[test]
fn cargo_is_its_own_registry_and_manager() {
    let cargo = ecosystem_profile("cargo").expect("cargo");
    let registry = cargo.registry.expect("cargo names a registry");
    assert_eq!(registry.id, "cargo");
    assert_eq!(registry.default_repository, Some("https://crates.io/"));
}

#[test]
fn registry_ids_are_purl_types_not_langbank_inventions() {
    // If an id here is not a purl type, `tools/sync-purl.py check` fails in CI.
    // Asserting a couple of the awkward ones guards the shape of the import.
    for id in [
        "golang",
        "cocoapods",
        "chrome-extension",
        "vscode-extension",
    ] {
        assert!(package_registry(id).is_some(), "{id} is a purl type");
    }
    assert!(package_registry("javascript").is_none(), "not a registry");
    assert!(package_registry("pnpm").is_none(), "pnpm is a manager");
}
