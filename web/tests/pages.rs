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

#[test]
fn every_page_links_the_stylesheet_rather_than_carrying_a_copy() {
    // 5 KB of CSS inlined into an 11 KB language page made nearly half of every
    // page a copy of the previous one, on a site meant to be clicked through.
    let path = pages::stylesheet_path();
    for (name, html) in [
        ("home", pages::home()),
        ("languages", pages::languages()),
        ("gaps", pages::gaps()),
        ("identify", pages::identify("src/main.rs", "")),
    ] {
        assert!(html.contains(path), "{name} does not link {path}");
        assert!(!html.contains("<style"), "{name} still carries inline CSS");
    }
}

#[test]
fn the_stylesheet_url_changes_when_the_stylesheet_does() {
    // The URL is promised `immutable` for a year, which is only safe because a
    // changed file means a changed name. This checks the hash is of the
    // content and not of, say, the length.
    let path = pages::stylesheet_path();
    assert!(
        path.starts_with("/site.") && path.ends_with(".css"),
        "{path}"
    );

    let hash = |css: &str| {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in css.as_bytes() {
            h ^= u64::from(*byte);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        format!("/site.{h:016x}.css")
    };
    assert_eq!(
        hash(pages::CSS),
        path,
        "the served hash is not of the served bytes"
    );

    // Same length, different contents.
    let swapped = pages::CSS.replacen("--bg", "--gb", 1);
    assert_eq!(swapped.len(), pages::CSS.len());
    assert_ne!(hash(&swapped), path, "an edit did not change the URL");
}

#[test]
fn the_front_page_leads_with_what_is_known_rather_than_with_prose() {
    let html = pages::home();
    // Every facet, and its purpose, on the front page — the thing a reader
    // wants first is what is actually in here.
    for facet in langbank::Facet::ALL {
        assert!(html.contains(facet.name()), "home omits {}", facet.name());
        assert!(
            html.contains(facet.purpose()),
            "home omits why {} matters",
            facet.name()
        );
    }
    // And the shape of it: most languages know exactly one thing.
    let one = langbank::distribution()[1];
    assert!(
        html.contains(&one.to_string()),
        "home omits the 1-of-8 count"
    );
}

#[test]
fn the_index_shows_how_much_is_known_about_each_language() {
    let html = pages::languages();
    // Eight cells per language, filled ones marked.
    assert!(html.contains("class=\"marks\""), "no per-language marks");
    assert!(html.contains("<i class=\"on\""), "no filled marks");
    // A well-covered language and a bare one both appear, with their counts in
    // the title text — which is what makes the column worth scanning.
    assert!(
        html.contains("nothing but a name"),
        "no bare language is labelled"
    );
}

#[test]
fn coverage_agrees_with_what_the_tool_reports() {
    // The site and `langbank-sync coverage` read the same function, so this is
    // really a check that the page renders the figures it was given rather than
    // recomputing them differently.
    let html = pages::coverage();
    for (facet, have) in langbank::Facet::ALL.into_iter().zip(langbank::coverage()) {
        assert!(
            html.contains(&format!("<td class=\"num\">{have}</td>")),
            "coverage page omits {} = {have}",
            facet.name()
        );
    }
}
