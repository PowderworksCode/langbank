//! Langbank carries the languages, and the contests that come with them.
//!
//! The data is langbank's own — one file per language, thin ones alongside
//! richly modelled ones, all in the same shape. What keeps it honest against
//! upstream is `tools/sync-linguist.py check`, which CI runs; what these tests
//! keep honest is that absorbing 800 languages did not change an answer
//! langbank already gave.

use std::path::Path;

use langbank::*;

#[test]
fn the_registry_carries_every_language_file() {
    assert!(
        language_profiles().len() > 800,
        "the full set is registered"
    );
    // and the richly modelled ones are still in there, in the same shape
    assert!(
        language_profile("rust")
            .and_then(|p| p.conventions)
            .is_some()
    );
    assert!(
        language_profile("typescript")
            .and_then(|p| p.conventions)
            .is_some()
    );
}

#[test]
fn depth_is_read_off_the_data_not_off_a_tier() {
    // There is no curated/imported flag: a language is as modelled as its data.
    let modelled = language_profiles()
        .iter()
        .filter(|profile| profile.conventions.is_some() || !profile.facets.is_empty())
        .count();
    assert!((15..60).contains(&modelled), "modelled today: {modelled}");
    let thin = language_profile("cobol").expect("cobol");
    assert!(thin.conventions.is_none());
    assert!(
        !thin.extensions.is_empty(),
        "but it can still be recognised"
    );
}

#[test]
fn a_language_records_where_its_facts_came_from() {
    assert_eq!(
        language_profile("cobol").expect("cobol").sources,
        &["linguist"]
    );
    // hand-written entries carry no source, because nobody imported them
    assert!(language_profile("rust").expect("rust").sources.is_empty());
}

#[test]
fn absorbing_upstream_changed_no_answer_langbank_already_gave() {
    // Every one of these resolved before the absorb and must still resolve the
    // same way, even though upstream hands the same extension to other
    // languages: `.rs` to RenderScript and XML, `.h` to C++ and Objective-C.
    for (extension, language) in [
        ("rs", "rust"),
        ("ts", "typescript"),
        ("tsx", "typescript"),
        ("h", "c"),
        ("md", "markdown"),
        ("json", "json"),
        ("php", "php"),
        ("cs", "c-sharp"),
        ("yml", "yaml"),
        ("mk", "make"),
    ] {
        assert_eq!(
            language_profile_for_extension(extension).map(|profile| profile.id),
            Some(language),
            ".{extension}"
        );
        assert!(
            languages_claiming_extension(extension).len() > 1,
            ".{extension} should be contested, or this test proves nothing"
        );
    }
    assert_eq!(
        detect_language(Path::new("src/main.rs"), None).map(|d| d.language),
        Some(LanguageId::new("rust"))
    );
}

#[test]
fn a_contest_nobody_claims_declines_to_answer() {
    // `.inc` belongs to twelve languages and no one of them owns it. Guessing
    // is a wrong answer where declining is merely an unhelpful one.
    for extension in ["inc", "spec", "fcgi", "cgi"] {
        assert!(
            language_profile_for_extension(extension).is_none(),
            ".{extension} resolved to {:?} but nothing claims it",
            language_profile_for_extension(extension).map(|p| p.id)
        );
        assert!(languages_claiming_extension(extension).len() > 1);
    }
    assert!(detect_language(Path::new("legacy.inc"), None).is_none());
}

#[test]
fn an_uncontested_language_resolves_without_claiming_anything() {
    for (extension, language) in [("cob", "cobol"), ("erl", "erlang"), ("hs", "haskell")] {
        let profile = language_profile_for_extension(extension).expect(extension);
        assert_eq!(profile.id, language);
        assert!(
            profile.primary_extensions.is_empty(),
            "{language} wins {extension} by being alone, not by claiming it"
        );
    }
}

#[test]
fn no_two_languages_claim_the_same_extension_as_primary() {
    // The registry asserts this at load; asserting it here names the failure.
    let mut claimed: Vec<(&str, &str)> = Vec::new();
    for profile in language_profiles() {
        for extension in profile.primary_extensions {
            assert!(
                profile.extensions.contains(extension),
                "{} claims {extension} as primary without declaring it",
                profile.id
            );
            claimed.push((extension, profile.id));
        }
    }
    claimed.sort();
    for pair in claimed.windows(2) {
        assert_ne!(
            pair[0].0, pair[1].0,
            "{} and {} both claim it",
            pair[0].1, pair[1].1
        );
    }
}
