//! The pages render, and render safely.
//!
//! These call the renderers directly rather than going over HTTP: the routing
//! is four lines of axum and the interesting failures are all in what the HTML
//! says. Escaping is checked here because `/identify` reflects a visitor's
//! filename and file body straight back into the page.

use langbank_web as pages;

#[test]
fn every_page_renders_and_closes_its_document() {
    let pages = [
        ("home", pages::home()),
        ("languages", pages::languages()),
        ("ecosystems", pages::ecosystems()),
        ("toolchains", pages::toolchains()),
        ("registries", pages::registries()),
        ("tools", pages::tools()),
        ("gaps", pages::gaps()),
    ];
    for (name, html) in pages {
        assert!(
            html.to_lowercase().starts_with("<!doctype html>"),
            "{name} does not open with a doctype"
        );
        assert!(html.contains("</footer>"), "{name} was cut short");
        assert!(html.len() > 2_000, "{name} rendered {} bytes", html.len());
    }
}

#[test]
fn the_counts_on_the_front_page_come_from_the_registry() {
    let html = pages::home();
    for n in [
        langbank::language_profiles().len(),
        langbank::ecosystem_profiles().len(),
        langbank::toolchains().len(),
        langbank::package_registries().len(),
    ] {
        assert!(html.contains(&format!("<b>{n}</b>")), "missing count {n}");
    }
}

#[test]
fn a_language_page_exists_for_every_language() {
    for language in langbank::language_profiles() {
        let html =
            pages::language(language.id).unwrap_or_else(|| panic!("{} has no page", language.id));
        assert!(html.contains("</footer>"), "{} was cut short", language.id);
    }
}

#[test]
fn an_unknown_language_has_no_page_rather_than_an_empty_one() {
    assert!(pages::language("not-a-language").is_none());
}

#[test]
fn a_visitors_filename_cannot_become_markup() {
    let hostile = "\"><script>alert(1)</script>";
    let html = pages::identify(hostile, hostile);
    assert!(!html.contains("<script>"), "a script tag survived escaping");
    assert!(!html.contains("alert(1)</"), "markup survived escaping");
    assert!(
        html.contains("&lt;script&gt;"),
        "the input was dropped, not escaped"
    );
}

#[test]
fn langbank_own_data_containing_angle_brackets_is_escaped_too() {
    // XML's comment markers and C++'s `template <` would both break the page if
    // the data were trusted just because langbank authored it.
    let html = pages::language("xml").expect("xml");
    assert!(
        !html.contains("<!--"),
        "an XML comment marker rendered as markup"
    );
    assert!(html.contains("&lt;!--"));
}

#[test]
fn identify_reports_the_evidence_and_not_only_the_answer() {
    let html = pages::identify("legacy.h", "@interface Greeter : NSObject\n@end\n");
    assert!(html.contains("Objective-C"), "wrong language");
    assert!(
        html.contains("rule 1 of 3"),
        "the rule that fired is not named"
    );

    let by_name = pages::identify("legacy.h", "");
    assert!(
        by_name.contains("claims it first"),
        "the fallback is not explained"
    );
}

#[test]
fn a_paste_larger_than_the_limit_is_truncated_rather_than_refused() {
    // The truncation is by byte offset and the input is not; slicing a `String`
    // mid-character panics, so this is the case that would take the site down.
    let huge = "\u{1F525}".repeat(80_000);
    let html = pages::identify("legacy.h", &huge);
    assert!(
        html.contains("</footer>"),
        "the page did not finish rendering"
    );
}
