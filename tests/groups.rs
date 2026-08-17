//! Languages counted under other languages.
//!
//! linguist's `group` is a statistical rollup — BibTeX under TeX, an APKBUILD
//! under Shell — and not a claim about language design. It is worth carrying
//! because a consumer totalling bytes by language wants exactly that answer,
//! and worth keeping apart from `supersedes`, which says one language replaced
//! another.

use langbank::{language_profile, language_profiles};

#[test]
fn no_language_is_grouped_under_itself() {
    // Two linguist entries can slug to the same langbank id — Cairo and Cairo
    // Zero both become `cairo` — and importing that blind produces a language
    // that is its own parent.
    for profile in language_profiles() {
        if let Some(parent) = profile.groups_under {
            assert!(
                !std::ptr::eq(parent, *profile),
                "{} is grouped under itself",
                profile.id
            );
        }
    }
}

#[test]
fn grouping_terminates() {
    // A cycle would hang any consumer that rolls totals up to the root.
    for profile in language_profiles() {
        let mut seen = vec![profile.id];
        let mut current = *profile;
        while let Some(parent) = current.groups_under {
            assert!(
                !seen.contains(&parent.id),
                "grouping cycles: {seen:?} then {}",
                parent.id
            );
            seen.push(parent.id);
            current = parent;
            assert!(seen.len() < 16, "grouping runs too deep: {seen:?}");
        }
    }
}

#[test]
fn a_group_is_not_the_same_claim_as_superseding() {
    // TypeScript replaced JavaScript; it is not counted under it.
    let typescript = language_profile("typescript").expect("typescript");
    let javascript = language_profile("javascript").expect("javascript");
    assert!(typescript.supersedes(javascript));
    assert!(typescript.groups_under.is_none());

    // BibTeX did not replace TeX; its bytes count as TeX.
    let bibtex = language_profile("bibtex").expect("bibtex");
    assert_eq!(bibtex.groups_under.map(|parent| parent.id), Some("tex"));
    assert!(!bibtex.supersedes(language_profile("tex").expect("tex")));
}

#[test]
fn a_parent_can_name_its_dialects() {
    let tex = language_profile("tex").expect("tex");
    let dialects: Vec<&str> = tex.dialects().iter().map(|entry| entry.id).collect();
    assert!(dialects.contains(&"bibtex"), "{dialects:?}");
    // And the relation is the same one read from the other end.
    for dialect in tex.dialects() {
        assert_eq!(dialect.groups_under.map(|parent| parent.id), Some("tex"));
    }
}
