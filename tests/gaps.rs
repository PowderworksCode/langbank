//! Absence with a reason.
//!
//! Langbank declines to answer a lot of questions, and until now every reason
//! looked identical from the outside. A gap distinguishes a fact nobody has
//! recorded from one two sources contradict each other about — which is the
//! difference between "someone should do the work" and "the work was done and
//! the answer is genuinely disputed".

use langbank::*;

#[test]
fn a_declined_answer_can_now_explain_itself() {
    // `.luau` resolves to nothing. Before, that was indistinguishable from an
    // extension nobody had heard of.
    assert!(language_profile_for_extension("luau").is_none());
    let gap = gap("luau", "extension-owner").expect("a recorded reason");
    assert_eq!(gap.reason, GapReason::SourcesDisagree);
    assert!(
        gap.note.contains("tokei") && gap.note.contains("scc"),
        "{}",
        gap.note
    );
}

#[test]
fn an_uncorroborated_claim_is_recorded_rather_than_acted_on() {
    // One corpus names an owner and the other is silent. Not enough to overrule
    // the other claimants, and too much to throw away.
    let uncorroborated = gaps()
        .iter()
        .filter(|gap| gap.reason == GapReason::Uncorroborated)
        .count();
    assert!(
        uncorroborated > 20,
        "{uncorroborated} single-source claims kept"
    );
    for gap in gaps().iter().filter(|g| g.facet == "extension-owner") {
        assert!(
            language_profile_for_extension(gap.subject).is_none(),
            ".{} has a recorded gap and yet resolves",
            gap.subject
        );
    }
}

#[test]
fn a_disputed_fact_is_absent_from_the_data_and_present_in_the_gaps() {
    // Lua is the case: scc knows six block-comment forms for its long brackets
    // and tokei knows one, so langbank carries neither.
    assert!(comment_syntax("lua").is_none());
    let gap = gap("lua", "comment-syntax").expect("lua is disputed");
    assert_eq!(gap.reason, GapReason::SourcesDisagree);
    assert!(gap.note.contains("block"), "{}", gap.note);
}

#[test]
fn every_gap_is_about_something_langbank_actually_carries() {
    for gap in gaps() {
        let known = language_profile(gap.subject).is_some()
            || !languages_claiming_extension(gap.subject).is_empty();
        assert!(
            known,
            "gap names {:?}, which langbank does not carry",
            gap.subject
        );
        assert!(!gap.note.is_empty(), "{} has no note", gap.subject);
    }
}

#[test]
fn gaps_can_be_asked_for_by_subject() {
    let about_lua = gaps_for("lua");
    assert!(about_lua.iter().any(|gap| gap.facet == "comment-syntax"));
}

#[test]
fn an_alternate_manifest_is_not_a_disambiguator() {
    // These were one field until they were two. `build.gradle.kts` is simply
    // the other way to spell gradle's manifest; `pnpm-workspace.yaml` decides
    // which of four managers owns a `package.json`. Only the second is a
    // selector, and conflating them left a consumer unable to tell.
    let gradle = ecosystem_profile("gradle").expect("gradle");
    assert_eq!(gradle.alternate_manifests, &["build.gradle.kts"]);
    assert!(gradle.selector_files.is_empty());

    let pnpm = ecosystem_profile("pnpm").expect("pnpm");
    assert_eq!(pnpm.selector_files, &["pnpm-workspace.yaml"]);
    assert!(pnpm.alternate_manifests.is_empty());

    // zig is the third case the old field was quietly carrying: its manifest
    // is optional, so the build script is what identifies the ecosystem at all.
    let zig = ecosystem_profile("zig").expect("zig");
    assert_eq!(zig.selector_files, &["build.zig"]);
    assert!(zig.alternate_manifests.is_empty());
}
