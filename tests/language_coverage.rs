//! What langbank knows per language, and the shape of what it does not.
//!
//! `tools/coverage-report.py` counts this properly. These assert the parts that
//! should not silently regress — and one thing that should stay *absent*.

use langbank::*;

fn ecosystems_for(language: &LanguageProfile) -> Vec<&'static EcosystemProfile> {
    ecosystem_profiles()
        .iter()
        .copied()
        .filter(|ecosystem| ecosystem.implies_language(language))
        .collect()
}

#[test]
fn the_languages_people_actually_use_have_a_package_manager() {
    // Python had none at all until this pass, which is the sort of hole a
    // registry claiming completeness should not have.
    for id in [
        "python", "ruby", "perl", "r", "haskell", "julia", "php", "java", "c-sharp", "go", "rust",
        "swift", "dart", "elixir", "scala", "kotlin",
    ] {
        let language = language_profile(id).unwrap_or_else(|| panic!("{id}"));
        assert!(
            !ecosystems_for(language).is_empty(),
            "{id} has no package ecosystem"
        );
    }
}

#[test]
fn several_managers_over_one_registry_is_now_the_common_case() {
    // The registry/manager split earning its keep a third time: Python has
    // five managers, four of which publish into pypi.
    let python = language_profile("python").expect("python");
    let managers = ecosystems_for(python);
    assert!(
        managers.len() >= 4,
        "python has {} managers",
        managers.len()
    );
    let pypi = package_registry("pypi").expect("pypi");
    let into_pypi = managers
        .iter()
        .filter(|eco| {
            eco.registry
                .is_some_and(|registry| std::ptr::eq(registry, pypi))
        })
        .count();
    assert!(into_pypi >= 4, "{into_pypi} publish into pypi");
    // and they are told apart by their lockfiles, as npm's four are
    assert_eq!(
        ecosystem_profile("poetry").expect("poetry").lockfiles,
        &["poetry.lock"]
    );
    assert_eq!(ecosystem_profile("uv").expect("uv").lockfiles, &["uv.lock"]);
}

#[test]
fn c_and_cpp_share_their_managers() {
    for id in ["conan", "vcpkg"] {
        let ecosystem = ecosystem_profile(id).unwrap_or_else(|| panic!("{id}"));
        assert!(ecosystem.implies_language(language_profile("c").expect("c")));
        assert!(ecosystem.implies_language(language_profile("cpp").expect("cpp")));
    }
}

#[test]
fn every_ecosystem_registry_resolves_to_a_purl_type() {
    for ecosystem in ecosystem_profiles() {
        if let Some(registry) = ecosystem.registry {
            assert!(
                package_registry(registry.id).is_some(),
                "{} names {} which is not a purl type",
                ecosystem.id,
                registry.id
            );
        }
    }
}

#[test]
fn structured_code_was_not_pasted_onto_every_programming_language() {
    // It would be true of all of them and therefore say nothing `role` does not
    // already say. A facet carrying no information is worse than an absent one,
    // because it looks like knowledge.
    let programming = language_profiles()
        .iter()
        .filter(|profile| profile.role == LanguageRole::Programming)
        .count();
    let with_facet = language_profiles()
        .iter()
        .filter(|profile| {
            profile
                .facets
                .iter()
                .any(|facet| facet.id == "structured-code")
        })
        .count();
    assert!(
        with_facet < programming / 4,
        "{with_facet} of {programming} carry structured-code — has it been bulk-filled?"
    );
}

#[test]
fn a_language_nothing_can_recognise_is_still_carried() {
    // linguist identifies REPL transcripts and OpenAPI documents by reading
    // them. Langbank does not read files, so it carries the language and can
    // find it by name only — a limit worth stating rather than hiding.
    for id in ["python-console", "julia-repl", "openapi-specification-v3"] {
        let language = language_profile(id).unwrap_or_else(|| panic!("{id}"));
        assert!(language.extensions.is_empty(), "{id}");
        assert!(language.filenames.is_empty(), "{id}");
        assert!(language.shebangs.is_empty(), "{id}");
    }
}
