//! Bulk-imported detection facts, and the precedence rules that keep them from
//! displacing anything somebody actually checked.
//!
//! The import is a floor. It should let a walker name a file it could not name
//! before, and it should never change an answer a curated profile already gave.

use std::path::Path;

use langbank::*;

fn curated() -> impl Iterator<Item = &'static LanguageProfile> {
    language_profiles()
        .iter()
        .copied()
        .filter(|profile| profile.provenance.is_curated())
}

fn imported() -> impl Iterator<Item = &'static LanguageProfile> {
    language_profiles()
        .iter()
        .copied()
        .filter(|profile| !profile.provenance.is_curated())
}

#[test]
fn the_import_is_a_floor_not_an_override() {
    assert_eq!(curated().count(), 29);
    assert!(imported().count() > 700, "the bulk import landed");
    // no imported profile may take an id a curated one already holds
    for profile in imported() {
        assert!(
            language_profile(profile.id).is_some_and(|found| std::ptr::eq(found, profile)),
            "{} resolves to something other than itself",
            profile.id
        );
    }
}

#[test]
fn imported_languages_say_where_they_came_from() {
    let go = language_profile("cobol").expect("an imported language");
    match go.provenance {
        LanguageProvenance::Imported { upstream } => {
            assert!(
                upstream.starts_with("github-linguist/linguist@"),
                "{upstream}"
            );
        }
        LanguageProvenance::Curated => panic!("cobol is not curated"),
    }
    assert_eq!(
        language_profile("rust").expect("rust").provenance,
        LanguageProvenance::Curated
    );
}

#[test]
fn imported_languages_carry_detection_and_nothing_else() {
    // This is the whole reason provenance exists: absence of a convention on an
    // imported language means nobody looked, not that the language has none.
    for profile in imported() {
        assert!(
            profile.conventions.is_none(),
            "{} has conventions",
            profile.id
        );
        assert!(profile.facets.is_empty(), "{} has facets", profile.id);
        assert!(
            profile.comments.is_none(),
            "{} has comment syntax",
            profile.id
        );
        assert!(profile.supersedes.is_empty(), "{} supersedes", profile.id);
    }
    // and the curated ones still do carry them
    assert!(
        language_profile("rust")
            .and_then(|p| p.conventions)
            .is_some()
    );
}

#[test]
fn curated_answers_are_unchanged_by_the_import() {
    // Every extension a curated profile claims must still resolve to it, even
    // though linguist claims all 52 of them too.
    for profile in curated() {
        for extension in profile.extensions {
            let resolved = language_profile_for_extension(extension)
                .unwrap_or_else(|| panic!("{extension} resolves to nothing"));
            assert!(
                std::ptr::eq(resolved, profile),
                "{extension} resolved to {} instead of {}",
                resolved.id,
                profile.id
            );
        }
    }
    assert_eq!(
        detect_language(Path::new("src/main.rs"), None).map(|d| d.language),
        Some(LanguageId::new("rust"))
    );
    assert_eq!(
        detect_language(Path::new("Dockerfile"), None).map(|d| d.language),
        Some(LanguageId::new("dockerfile"))
    );
}

#[test]
fn an_unambiguous_imported_extension_now_resolves() {
    // none of these were known before the import
    for (extension, language) in [("cob", "cobol"), ("erl", "erlang"), ("hs", "haskell")] {
        assert_eq!(
            language_profile_for_extension(extension).map(|profile| profile.id),
            Some(language),
            "{extension}"
        );
    }
}

#[test]
fn an_ambiguous_imported_extension_declines_to_answer() {
    // `.inc` is claimed by a dozen languages and `.m` by seven. Choosing
    // between them without reading the file would be a confident wrong answer,
    // so detection returns nothing and the candidates stay available.
    for extension in ["inc", "m", "cls"] {
        assert!(
            language_profile_for_extension(extension).is_none(),
            "{extension} should be ambiguous, got {:?}",
            language_profile_for_extension(extension).map(|profile| profile.id)
        );
        assert!(
            languages_claiming_extension(extension).len() > 1,
            "{extension} should have several claimants"
        );
    }
    assert!(detect_language(Path::new("legacy.inc"), None).is_none());
}

#[test]
fn a_curated_language_wins_an_extension_that_is_otherwise_ambiguous() {
    // linguist gives `.h` to C, C++ and Objective-C. C is curated here, so the
    // ambiguity resolves rather than declining.
    let claimants = languages_claiming_extension("h");
    assert!(
        claimants.len() > 1,
        "h is contested: {:?}",
        claimants.iter().map(|p| p.id).collect::<Vec<_>>()
    );
    assert_eq!(language_profile_for_extension("h").map(|p| p.id), Some("c"));
}
