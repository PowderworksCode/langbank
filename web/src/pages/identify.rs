//! The rules, run against whatever a visitor pastes in.
//!
//! This is the only page that renders input it did not author. Every value from
//! the request is interpolated through maud, which escapes it — there is no
//! call site here that could forget. Nothing is stored and nothing is logged;
//! the answer is computed from the statics and thrown away.

use crate::render::{code, link, page};
use langbank_detect::{Evidence, Undecided, identify};
use maud::{Markup, html};
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Query {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub content: String,
}

/// Enough to fire a rule, and not enough to be worth a limit anyone would hit.
/// A rule that has not matched in 64 KiB is not going to.
const LIMIT: usize = 64 * 1024;

/// Truncate on a character boundary. Slicing a `String` at a byte offset panics
/// when the offset lands inside a multi-byte character, and a visitor pasting
/// 64 KiB of anything non-ASCII would land there eventually.
fn clamp(text: &str) -> &str {
    if text.len() <= LIMIT {
        return text;
    }
    let mut end = LIMIT;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn how(evidence: &Evidence, language: &langbank::LanguageProfile) -> Markup {
    match evidence {
        Evidence::Filename(name) => html! {
            "The whole filename " (code(name)) " is claimed, so nothing had to be read."
        },
        Evidence::Extension(extension) => {
            let claimants = langbank::languages_claiming_extension(extension).len();
            let dotted = code(&format!(".{extension}"));
            html! {
                @if claimants > 1 {
                    (claimants) " languages claim " (dotted) ", and " (language.display_name)
                    " claims it first — the answer for a caller that has not opened the
                     file. Paste some content below and the content rules decide instead."
                } @else {
                    (dotted) " is claimed by one language, so the name is the answer."
                }
            }
        }
        Evidence::Shebang(line) => html! {
            "The first line " (code(line))
            " names the interpreter; the filename was not needed."
        },
        Evidence::Content { extension, rule } => {
            let total = langbank::disambiguation_for(extension)
                .map(|d| d.rules.len())
                .unwrap_or(0);
            html! {
                "The name could not settle " (code(&format!(".{extension}")))
                " — rule " (rule + 1) " of " (total) " for it matched the content."
            }
        }
    }
}

/// The languages contesting an extension, linked where langbank carries a page
/// for one. Its own function because the alternative is four levels of macro
/// inside the branch that uses it, and the branch is hard enough to read.
fn claimant_list(claimants: &[&str]) -> Markup {
    html! {
        @for (index, id) in claimants.iter().enumerate() {
            @if index > 0 { ", " }
            @match langbank::language_profile(id) {
                Some(language) => (link(&format!("/languages/{id}"), language.display_name)),
                None => (id),
            }
        }
    }
}

fn verdict(query: &Query) -> Markup {
    let path = query.path.trim();
    if path.is_empty() {
        return html! {};
    }
    let content = query.content.trim_end_matches('\n');
    let content = (!content.is_empty()).then(|| clamp(content));

    match identify(path, content) {
        Ok(found) => html! {
            div.verdict {
                h3 { (link(&format!("/languages/{}", found.language.id),
                           found.language.display_name)) }
                p { (how(&found.evidence, found.language)) }
                p { (format!("{:?}", found.evidence)) }
            }
        },
        Err(Undecided::Unknown) => html! {
            div.verdict {
                h3 { "Not known" }
                p {
                    "No language claims that name, and no shebang was found. langbank
                     says nothing rather than guessing."
                }
            }
        },
        Err(Undecided::Contested {
            extension,
            claimants,
            had_rules,
        }) => html! {
            div.verdict {
                h3 { "Contested" }
                p {
                    (code(&format!(".{extension}"))) " is claimed by "
                    (claimant_list(&claimants))
                    ", and none of them claims it first. "
                    @if had_rules {
                        "There are content rules for it — paste the file below and they
                         will run."
                    } @else {
                        "langbank carries no content rules for this extension, so it
                         stays contested."
                    }
                }
            }
        },
    }
}

pub fn render(query: &Query) -> String {
    let rules: usize = langbank::disambiguations()
        .iter()
        .map(|d| d.rules.len())
        .sum();
    let examples = [
        ("src/main.rs", "a name that settles itself"),
        ("legacy.h", "three languages claim it"),
        ("man/git.1", "contested with no first claim"),
        ("scripts/deploy", "no extension at all"),
    ];

    let body = html! {
        h1 { "Identify a file" }
        p.lede {
            "langbank does not read files. It carries " (rules) " ordered rules across "
            (langbank::disambiguations().len()) " contested extensions so a caller
             holding the bytes can settle what a filename cannot — and reports which
             rule fired, not just the verdict."
        }

        (verdict(query))

        form method="post" action="/identify" {
            label for="path" { "Path or filename" }
            input id="path" name="path" value=(query.path) placeholder="src/main.rs" autofocus;
            label for="content" {
                "File content "
                span.none { "— optional; the rules only run when there is something to read" }
            }
            textarea id="content" name="content"
                     placeholder="#!/usr/bin/env bash\necho hi" { (query.content) }
            button type="submit" { "Identify" }
        }

        h2 { "Try these" }
        ul.grid {
            @for (path, label) in examples {
                li { (link(&format!("/identify?path={path}"), path)) " " small { (label) } }
            }
        }

        h2 { "Why the evidence matters" }
        p {
            "“This is C” and “this is C because rule 3 for " code { ".h" } " matched” are
             different claims. A consumer joining this answer to a parse tree or a
             compiler observation needs the second one: it can tell a confident answer
             from a fallback, and it can disagree with a specific rule rather than with
             langbank."
        }
        p { "The same call is available without the web page:" }
        pre { code {
"langbank_detect::identify(\"legacy.h\", Some(source))
// Ok(Identification { language: cpp, evidence: Content { extension: \"h\", rule: 1 } })"
        } }
    };
    page("Identify a file", &[("/", "langbank")], body)
}
