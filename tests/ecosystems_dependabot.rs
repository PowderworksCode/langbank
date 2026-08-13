//! Package ecosystems extracted from dependabot-core.
//!
//! Dependabot states its facts in Ruby, so they are read with Ripper — Ruby's
//! own parser — rather than by regex. The ecosystems that name a constant
//! instead of spelling a filename out are the reason: `composer` says
//! `PackageManager::MANIFEST_FILENAME` and `deno` says `MANIFEST_FILENAMES`,
//! and following those references took the yield from 16 ecosystems to 27.

use langbank::*;

#[test]
fn ecosystem_coverage_grew_past_the_javascript_and_rust_corner() {
    assert!(
        ecosystem_profiles().len() >= 19,
        "{}",
        ecosystem_profiles().len()
    );
    for id in [
        "bundler",
        "composer",
        "maven",
        "gradle",
        "hex",
        "pub",
        "swift-pm",
        "go-modules",
    ] {
        assert!(ecosystem_profile(id).is_some(), "{id}");
    }
}

#[test]
fn an_ecosystem_publishes_for_a_language_and_into_a_registry() {
    let bundler = ecosystem_profile("bundler").expect("bundler");
    assert!(bundler.implies_language(language_profile("ruby").expect("ruby")));
    assert_eq!(bundler.registry.map(|r| r.id), Some("gem"));
    assert_eq!(bundler.manifest, Some("Gemfile"));

    let go = ecosystem_profile("go-modules").expect("go modules");
    assert!(go.implies_language(language_profile("go").expect("go")));
    assert_eq!(go.registry.map(|r| r.id), Some("golang"));
    assert_eq!(go.manifest, Some("go.mod"));
}

#[test]
fn several_managers_can_publish_into_one_registry() {
    // maven, gradle and sbt are three build systems over one artifact
    // namespace, which is the same distinction npm/pnpm/yarn/bun made for
    // JavaScript and the reason registries are modelled apart from managers.
    let maven = package_registry("maven").expect("maven registry");
    for id in ["maven", "gradle", "sbt"] {
        let ecosystem = ecosystem_profile(id).unwrap_or_else(|| panic!("{id}"));
        let registry = ecosystem
            .registry
            .unwrap_or_else(|| panic!("{id} names no registry"));
        assert!(std::ptr::eq(registry, maven), "{id} publishes to maven");
    }
    // and they are still told apart by their manifests
    assert_eq!(
        ecosystem_profile("gradle").expect("gradle").manifest,
        Some("build.gradle")
    );
    assert_eq!(
        ecosystem_profile("sbt").expect("sbt").manifest,
        Some("build.sbt")
    );
}

#[test]
fn an_ecosystem_with_no_purl_type_says_so_rather_than_inventing_one() {
    // purl defines no type for Elm or Deno packages. The ecosystem is still
    // carried, because its manifest is a fact; it simply names no registry.
    for id in ["elm", "deno"] {
        let ecosystem = ecosystem_profile(id).unwrap_or_else(|| panic!("{id}"));
        assert!(ecosystem.registry.is_none(), "{id} should name no registry");
        assert!(ecosystem.manifest.is_some(), "{id} still has a manifest");
    }
}

#[test]
fn alternate_manifests_are_kept_rather_than_dropped() {
    // Langbank models one manifest per ecosystem, and dependabot accepts
    // several for some. The rest are recorded as selectors so nothing is lost.
    let bundler = ecosystem_profile("bundler").expect("bundler");
    assert_eq!(bundler.selector_files, &["gems.rb"]);
    let go = ecosystem_profile("go-modules").expect("go modules");
    assert_eq!(go.selector_files, &["go.work"]);
}

#[test]
fn lockfiles_were_taken_only_where_dependabot_names_them() {
    let composer = ecosystem_profile("composer").expect("composer");
    assert_eq!(composer.lockfiles, &["composer.lock"]);
    // maven's lockfile is not something dependabot declares, so none is claimed
    assert!(
        ecosystem_profile("maven")
            .expect("maven")
            .lockfiles
            .is_empty()
    );
}

#[test]
fn infrastructure_updaters_were_not_absorbed_as_package_ecosystems() {
    // dependabot also updates github actions, git submodules, devcontainers,
    // terraform and docker. None of those manages a language's packages.
    for id in [
        "github-actions",
        "git-submodules",
        "devcontainers",
        "terraform",
        "docker",
    ] {
        assert!(
            ecosystem_profile(id).is_none(),
            "{id} is not a package ecosystem"
        );
    }
}
