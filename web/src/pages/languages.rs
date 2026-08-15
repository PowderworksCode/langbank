//! The language index and one page per language.

use crate::render::{code, codes, escape, link, page, row};
use langbank::LanguageProfile;
use std::fmt::Write as _;

pub fn index() -> String {
    let mut all: Vec<&LanguageProfile> = langbank::language_profiles().to_vec();
    all.sort_by_key(|l| l.display_name.to_lowercase());

    let mut items = String::new();
    for language in &all {
        let hint = language
            .primary_extensions
            .first()
            .or_else(|| language.extensions.first())
            .map(|e| format!(" <small>.{}</small>", escape(e)))
            .unwrap_or_default();
        let _ = write!(
            items,
            "<li>{}{hint}</li>",
            link(
                &format!("/languages/{}", language.id),
                language.display_name
            )
        );
    }

    let body = format!(
        "<h1>{} languages</h1>\n\
         <p class=lede>Every language langbank carries, with the extension it claims \
         first. A language is here because a source named it, not because it seemed \
         important.</p>\n\
         <ul class=grid>{items}</ul>",
        all.len()
    );
    page("Languages", &[("/", "langbank")], &body)
}

/// One language. Rows with nothing in them are omitted rather than rendered
/// empty, so what langbank actually knows is visible at a glance.
pub fn detail(id: &str) -> Option<String> {
    let language = langbank::language_profile(id)?;

    let mut rows = String::new();
    let mut add = |label: &str, value: String| {
        let _ = write!(rows, "{}", row(label, value));
    };

    add("id", code(language.id));
    add("role", format!("{:?}", language.role).to_lowercase());
    if !language.primary_extensions.is_empty() {
        add(
            "claims first",
            codes(
                &language
                    .primary_extensions
                    .iter()
                    .map(|e| format!(".{e}"))
                    .collect::<Vec<_>>(),
            ),
        );
    }
    if !language.extensions.is_empty() {
        add(
            "extensions",
            codes(
                &language
                    .extensions
                    .iter()
                    .map(|e| format!(".{e}"))
                    .collect::<Vec<_>>(),
            ),
        );
    }
    if !language.filenames.is_empty() {
        add("filenames", codes(language.filenames));
    }
    if !language.shebangs.is_empty() {
        add("shebangs", codes(language.shebangs));
    }
    if !language.config_files.is_empty() {
        add("config files", codes(language.config_files));
    }
    if !language.facets.is_empty() {
        add(
            "facets",
            language
                .facets
                .iter()
                .map(|f| {
                    format!(
                        "<span class=tag title=\"{}\">{}</span>",
                        escape(f.description),
                        escape(f.id)
                    )
                })
                .collect::<Vec<_>>()
                .join(""),
        );
    }
    if !language.supersedes.is_empty() {
        add(
            "supersedes",
            language
                .supersedes
                .iter()
                .map(|s| link(&format!("/languages/{}", s.id), s.display_name))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    if let Some(comments) = language.comments {
        if !comments.line.is_empty() {
            add("line comment", codes(comments.line));
        }
        if !comments.block.is_empty() {
            add(
                "block comment",
                comments
                    .block
                    .iter()
                    .map(|(open, close)| format!("{} … {}", code(open), code(close)))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if !comments.documentation.is_empty() {
            add("doc comment", codes(comments.documentation));
        }
        if !comments.quotes.is_empty() {
            add(
                "string quotes",
                codes(
                    &comments
                        .quotes
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>(),
                ),
            );
        }
        if !comments.multi_quotes.is_empty() {
            add("multiline quotes", codes(comments.multi_quotes));
        }
    }

    let toolchains = langbank::toolchains_for(language);
    if !toolchains.is_empty() {
        add(
            "toolchains",
            toolchains
                .iter()
                .map(|t| link(&format!("/toolchains#{}", t.id), t.display_name))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    let ecosystems: Vec<String> = langbank::ecosystem_profiles()
        .iter()
        .filter(|e| e.implied_languages.iter().any(|l| l.id == language.id))
        .map(|e| link(&format!("/ecosystems#{}", e.id), e.display_name))
        .collect();
    if !ecosystems.is_empty() {
        add("ecosystems", ecosystems.join(", "));
    }

    // Contested extensions are the interesting part: they are where a name is
    // not an answer and the content rules take over.
    let contested: Vec<String> = language
        .extensions
        .iter()
        .filter(|e| langbank::languages_claiming_extension(e).len() > 1)
        .map(|e| {
            let others: Vec<String> = langbank::languages_claiming_extension(e)
                .iter()
                .filter(|l| l.id != language.id)
                .map(|l| escape(l.display_name))
                .collect();
            format!("{} with {}", code(&format!(".{e}")), others.join(", "))
        })
        .collect();
    if !contested.is_empty() {
        add("shares", contested.join("<br>"));
    }

    let gaps = langbank::gaps_for(language.id);
    let mut gap_html = String::new();
    if !gaps.is_empty() {
        gap_html.push_str("<h2>What is not known, and why</h2><div class=scroll><table><thead><tr><th>facet<th>reason<th>note</thead><tbody>");
        for gap in &gaps {
            let _ = write!(
                gap_html,
                "<tr><td>{}<td>{}<td>{}",
                code(gap.facet),
                escape(&format!("{:?}", gap.reason)),
                escape(gap.note)
            );
        }
        gap_html.push_str("</tbody></table></div>");
    }

    let body = format!(
        "<h1>{}</h1><dl>{rows}</dl>{gap_html}",
        escape(language.display_name)
    );
    Some(page(
        language.display_name,
        &[("/", "langbank"), ("/languages", "languages")],
        &body,
    ))
}
