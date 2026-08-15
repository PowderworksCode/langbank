//! Facets, artifacts, ecosystems and traversal, generated from `data/`.
//!
//! Same discipline as `generated_data.rs`: assert the facts the hand-written
//! Rust asserted, so a TOML file that silently loses a field fails here rather
//! than in a consumer.

use langbank::*;

#[test]
fn the_registries_are_the_expected_size() {
    assert_eq!(language_facets().len(), 3, "facets");
    assert_eq!(artifact_profiles().len(), 4, "artifacts");
    assert_eq!(ecosystem_profiles().len(), 19, "ecosystems");
    // target (cargo); node_modules, dist, build, .next, .turbo, coverage (npm);
    // .zig-cache, zig-out (zig)
    assert_eq!(traversal_directories().len(), 9, "traversal directories");
}

#[test]
fn facets_keep_their_descriptions() {
    let structured = language_facet("structured-code").expect("structured-code");
    assert_eq!(
        structured.description,
        "source expresses nested executable code structure"
    );
    assert!(language_facet("style-host").is_some());
    assert!(language_facet("component-host").is_some());
}

#[test]
fn artifacts_keep_every_field() {
    let napi = artifact_profile("napi").expect("napi");
    assert_eq!(napi.display_name, "Node native addon (napi-rs)");
    assert_eq!(napi.package_dependencies, &["@napi-rs/cli"]);
    assert_eq!(napi.package_script_signals, &["napi build"]);
    assert!(napi.project_facets.is_empty());

    // an artifact identified by a project facet rather than a dependency
    let site = artifact_profile("site").expect("site");
    assert_eq!(site.project_facets, &["static-site"]);
    assert!(site.package_script_signals.is_empty());

    let binary = artifact_profile("binary").expect("binary");
    assert_eq!(binary.package_script_signals, &["bun build --compile"]);
}

#[test]
fn ecosystems_imply_languages_by_pointer() {
    let cargo = ecosystem_profile("cargo").expect("cargo");
    let rust = language_profile("rust").expect("rust");
    assert!(cargo.implies_language(rust), "cargo implies rust");

    let npm = ecosystem_profile("npm").expect("npm");
    let javascript = language_profile("javascript").expect("javascript");
    assert!(npm.implies_language(javascript));
    assert!(!npm.implies_language(rust));
}

#[test]
fn ecosystems_keep_their_manifests_and_lockfiles() {
    let cargo = ecosystem_profile("cargo").expect("cargo");
    assert_eq!(cargo.manifest, Some("Cargo.toml"));
    assert_eq!(cargo.lockfiles, &["Cargo.lock"]);
    assert_eq!(cargo.gitignore_patterns, &["target/"]);
    assert_eq!(
        cargo.roles,
        &[EcosystemRole::PackageManager, EcosystemRole::BuildSystem]
    );
    assert_eq!(cargo.manifest_selection, ManifestSelection::Default);

    // the four JavaScript ecosystems share a manifest and are told apart by
    // their lockfiles, which is why manifest_selection exists at all
    let pnpm = ecosystem_profile("pnpm").expect("pnpm");
    assert_eq!(pnpm.manifest, Some("package.json"));
    assert_eq!(pnpm.lockfiles, &["pnpm-lock.yaml"]);
    assert_eq!(pnpm.selector_files, &["pnpm-workspace.yaml"]);
    assert_eq!(pnpm.manifest_selection, ManifestSelection::Lockfile);

    let bun = ecosystem_profile("bun").expect("bun");
    assert_eq!(bun.lockfiles, &["bun.lock", "bun.lockb"]);
    assert_eq!(
        bun.roles,
        &[EcosystemRole::PackageManager, EcosystemRole::Runtime]
    );
}

#[test]
fn pin_policies_survive_losing_their_shared_constant() {
    // pnpm, yarn and bun referenced npm's DEPENDENCY_PINS constant directly.
    // In data each states its own, so the thing to check is that they still
    // classify identically — the sharing was an implementation detail, the
    // behaviour is the contract.
    let npm = ecosystem_profile("npm")
        .and_then(|eco| eco.dependency_pins)
        .expect("npm pins");
    for id in ["pnpm", "yarn", "bun"] {
        let pins = ecosystem_profile(id)
            .and_then(|eco| eco.dependency_pins)
            .unwrap_or_else(|| panic!("{id} pins"));
        for (source, requirement) in [
            (DependencySource::Registry, Some("1.2.3")),
            (DependencySource::Registry, Some("^1.2.3")),
            (DependencySource::Git, Some("main")),
            (DependencySource::LocalPath, None),
        ] {
            assert_eq!(
                pins.classify(source, requirement),
                npm.classify(source, requirement),
                "{id} disagrees with npm on {source:?} {requirement:?}"
            );
        }
    }

    // cargo is the one that genuinely differs: `=1.2.3`, not a bare version.
    let cargo = ecosystem_profile("cargo")
        .and_then(|eco| eco.dependency_pins)
        .expect("cargo pins");
    assert_eq!(
        cargo.classify(DependencySource::Registry, Some("=1.2.3")),
        DependencyPinStatus::Pinned
    );
    assert_eq!(
        cargo.classify(DependencySource::Registry, Some("1.2.3")),
        DependencyPinStatus::Floating
    );
    assert_eq!(
        npm.classify(DependencySource::Registry, Some("1.2.3")),
        DependencyPinStatus::Pinned
    );
    assert!(cargo.advisory, "cargo pinning is advisory");
    assert!(!npm.advisory);
}

#[test]
fn traversal_markers_separate_unambiguous_names_from_ordinary_words() {
    let directories = traversal_directories();
    let find = |name: &str| {
        directories
            .iter()
            .find(|directory| directory.name == name)
            .unwrap_or_else(|| panic!("{name} registered"))
    };

    // unambiguous: node_modules is never anything else
    assert!(find("node_modules").markers.is_empty());
    // ambiguous: `build` and `target` are ordinary words and need a marker in
    // the ancestry before a walker may prune them
    assert_eq!(find("build").markers, &["package.json"]);
    assert_eq!(find("target").markers, &["Cargo.toml"]);
    assert_eq!(find("coverage").markers, &["package.json"]);
    assert!(
        directories
            .iter()
            .any(|directory| directory.name == ".turbo")
    );
}

/// Zig's build script is always present and its manifest is not, so the
/// selector cannot be the manifest.
///
/// Checked against two real trees: the Zig compiler's own source carries
/// `build.zig`, `build.zig.zon`, `.zig-cache/` and `zig-out/`; Bun 1.3.14
/// carries `build.zig` and no `build.zig.zon`. An ecosystem keyed on the
/// manifest would miss the largest Zig codebase there is.
#[test]
fn zig_selects_on_its_build_script_rather_than_its_manifest() {
    let zig = ecosystem_profile("zig").expect("zig registered");
    assert_eq!(zig.manifest, Some("build.zig.zon"));
    assert_eq!(zig.selector_files, &["build.zig"]);

    let language = language_profile("zig").expect("zig language");
    assert!(zig.implies_language(language));

    let generated: Vec<&str> = traversal_directories()
        .iter()
        .filter(|directory| directory.markers.contains(&"build.zig"))
        .map(|directory| directory.name)
        .collect();
    assert_eq!(generated, &[".zig-cache", "zig-out"]);
}
