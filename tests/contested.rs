//! Contested extensions, and the ones two independent corpora settled.
//!
//! An extension several languages claim resolves to nothing unless exactly one
//! claimant declares it primary. That is honest and unhelpful, so where tokei
//! and scc — measured independent, not one corpus wearing two hats — both name
//! the same claimant, the claim is taken. Where only one has an opinion, or the
//! two disagree, nothing is claimed: a primary is a decision about what a file
//! *is*, and one source is not enough to make it on langbank's behalf.

use langbank::*;

#[test]
fn corroborated_contests_now_resolve() {
    for (extension, language) in [
        ("as", "actionscript"),
        ("asm", "assembly"),
        ("ex", "elixir"),
        ("gd", "gdscript"),
        ("d", "d"),
    ] {
        assert!(
            languages_claiming_extension(extension).len() > 1,
            ".{extension} should be contested, or this proves nothing"
        );
        assert_eq!(
            language_profile_for_extension(extension).map(|profile| profile.id),
            Some(language),
            ".{extension}"
        );
    }
}

#[test]
fn a_contest_the_corpora_disagree_about_stays_unresolved() {
    // tokei calls `.luau` Lua and scc calls it Luau; tokei calls `.tmpl` templ
    // and scc calls it a Go template. Neither is obviously wrong, so langbank
    // declines rather than picking the one it happened to read first.
    for extension in ["luau", "tmpl"] {
        assert!(
            language_profile_for_extension(extension).is_none(),
            ".{extension} resolved to {:?} despite the corpora disagreeing",
            language_profile_for_extension(extension).map(|p| p.id)
        );
        assert!(languages_claiming_extension(extension).len() > 1);
    }
}

#[test]
fn plenty_remain_unresolved_and_that_is_the_honest_answer() {
    let contested_unresolved = language_profiles()
        .iter()
        .flat_map(|profile| profile.extensions.iter())
        .filter(|extension| {
            languages_claiming_extension(extension).len() > 1
                && language_profile_for_extension(extension).is_none()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        contested_unresolved.len() > 50,
        "{} still unresolved — if this collapsed, something started guessing",
        contested_unresolved.len()
    );
    // `.inc` is claimed by a dozen languages and no corpus settles it
    assert!(language_profile_for_extension("inc").is_none());
}

#[test]
fn a_primary_claim_is_always_declared_by_the_language_making_it() {
    for profile in language_profiles() {
        for extension in profile.primary_extensions {
            assert!(
                profile.extensions.contains(extension),
                "{} claims .{extension} as primary without declaring it",
                profile.id
            );
        }
    }
}

#[test]
fn settling_contests_disturbed_no_answer_that_already_worked() {
    for (extension, language) in [
        ("rs", "rust"),
        ("ts", "typescript"),
        ("h", "c"),
        ("md", "markdown"),
        ("json", "json"),
        ("php", "php"),
        ("yml", "yaml"),
    ] {
        assert_eq!(
            language_profile_for_extension(extension).map(|profile| profile.id),
            Some(language),
            ".{extension}"
        );
    }
}
