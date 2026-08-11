//! Comment syntax, reconciled from two independent corpora.
//!
//! tokei and scc are genuinely independent — on the 187 languages both carry
//! they agree on 77% of extension sets, 93% of line comments and 89% of block
//! comments, nowhere near the ~100% that would mean one corpus wearing two
//! hats. That is what makes their agreement evidence, and it is why the 21
//! languages they disagree about are left alone rather than guessed at.

use langbank::*;

fn with_comments() -> usize {
    language_profiles()
        .iter()
        .filter(|profile| profile.comments.is_some())
        .count()
}

#[test]
fn comment_coverage_grew_from_a_handful_to_a_quarter() {
    let covered = with_comments();
    assert!(covered > 200, "comment syntax for {covered} languages");
    assert!(
        covered < language_profiles().len(),
        "and it is not claimed for languages nobody has data on"
    );
}

#[test]
fn hand_written_comment_syntax_was_not_overwritten() {
    // These predate the corpora and a corpus does not get to overrule them.
    let rust = comment_syntax("rust").expect("rust");
    assert_eq!(rust.line, &["//"]);
    assert_eq!(rust.quotes, &['"'], "rust has no single-quoted strings");
    assert!(rust.documentation.contains(&"///"));

    let python = comment_syntax("python").expect("python");
    assert!(python.multi_quotes.contains(&"\"\"\""));
}

#[test]
fn absorbed_languages_carry_line_and_block_comments() {
    let haskell = comment_syntax("haskell").expect("haskell");
    assert_eq!(haskell.line, &["--"]);
    assert!(haskell.block.iter().any(|(open, _)| *open == "{-"));

    let erlang = comment_syntax("erlang").expect("erlang");
    assert_eq!(erlang.line, &["%"]);

    let elixir = comment_syntax("elixir").expect("elixir");
    assert_eq!(elixir.line, &["#"]);
}

#[test]
fn tables_are_shared_rather_than_duplicated_per_language() {
    // Hundreds of languages comment with `//` and `/* */`; they must resolve to
    // one static rather than to hundreds of equal ones.
    let c = comment_syntax("c").expect("c");
    let alike = language_profiles()
        .iter()
        .filter(|profile| profile.comments.is_some_and(|table| std::ptr::eq(table, c)))
        .count();
    assert!(
        alike > 5,
        "the C-style table is shared by {alike} languages"
    );
}

#[test]
fn languages_the_corpora_disagree_about_are_left_alone() {
    // tokei and scc differ on these, so langbank says nothing rather than
    // picking a side. Absence here is a recorded refusal, not an oversight.
    // Lua is the clearest case: scc knows its `--[==[` long-bracket forms and
    // records six block variants where tokei records one. Neither is wrong and
    // picking the smaller answer would quietly lose the difference.
    for id in ["lua", "ats", "c3", "factor"] {
        assert!(
            comment_syntax(id).is_none(),
            "{id} is disputed between the corpora and should carry no syntax"
        );
    }
}

#[test]
fn an_extension_can_be_an_emoji() {
    // Mojo really does accept `.🔥`, and it is the case that proves the data
    // files are UTF-8 rather than ASCII with escapes.
    let mojo = language_profile("mojo").expect("mojo");
    assert!(mojo.extensions.contains(&"🔥"), "{:?}", mojo.extensions);
    assert_eq!(
        language_profile_for_extension("🔥").map(|p| p.id),
        Some("mojo")
    );
    assert!(!mojo.extensions.iter().any(|e| e.contains("\\u")));
}

#[test]
fn whole_filenames_were_refused_as_extensions() {
    // Both corpora list `cmakelists.txt` among cmake's "extensions". Langbank
    // keeps filenames and suffixes apart, so it must not have been absorbed.
    let cmake = language_profile("cmake").expect("cmake");
    assert!(!cmake.extensions.contains(&"cmakelists.txt"));
    assert!(
        cmake
            .filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("CMakeLists.txt"))
    );
    let meson = language_profile("meson").expect("meson");
    assert!(!meson.extensions.contains(&"meson.build"));
}
