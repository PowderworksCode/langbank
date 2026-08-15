//! The rules, run against whatever a visitor pastes in.
//!
//! This is the only page with a form, and the only one that renders input it did
//! not author — every value from the request goes through `escape` on the way
//! back out. Nothing is stored and nothing is logged; the answer is computed
//! from the statics and thrown away.

use crate::render::{code, escape, link, page};
use langbank_detect::{Evidence, Undecided, identify};
use serde::Deserialize;
use std::fmt::Write as _;

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

fn verdict(query: &Query) -> String {
    let path = query.path.trim();
    if path.is_empty() {
        return String::new();
    }
    let content = query.content.trim_end_matches('\n');
    let content = if content.is_empty() {
        None
    } else {
        Some(clamp(content))
    };

    match identify(path, content) {
        Ok(found) => {
            let how = match &found.evidence {
                Evidence::Filename(name) => format!(
                    "The whole filename {} is claimed, so nothing had to be read.",
                    code(name)
                ),
                Evidence::Extension(ext) => {
                    let claimants = langbank::languages_claiming_extension(ext).len();
                    if claimants > 1 {
                        format!(
                            "{} languages claim {}, and {} claims it first — the answer for a \
                             caller that has not opened the file. Paste some content below and \
                             the content rules decide instead.",
                            claimants,
                            code(&format!(".{ext}")),
                            escape(found.language.display_name),
                        )
                    } else {
                        format!(
                            "{} is claimed by one language, so the name is the answer.",
                            code(&format!(".{ext}"))
                        )
                    }
                }
                Evidence::Shebang(line) => format!(
                    "The first line {} names the interpreter; the filename was not needed.",
                    code(line)
                ),
                Evidence::Content { extension, rule } => {
                    let total = langbank::disambiguation_for(extension)
                        .map(|d| d.rules.len())
                        .unwrap_or(0);
                    format!(
                        "The name could not settle {} — rule {} of {} for it matched the content.",
                        code(&format!(".{extension}")),
                        rule + 1,
                        total
                    )
                }
            };
            format!(
                "<div class=verdict><h3>{}</h3><p>{how}</p><p>{}</p></div>",
                link(
                    &format!("/languages/{}", found.language.id),
                    found.language.display_name
                ),
                escape(&format!("{:?}", found.evidence))
            )
        }
        Err(Undecided::Unknown) => {
            "<div class=verdict><h3>Not known</h3><p>No language claims that name, and no \
             shebang was found. langbank says nothing rather than guessing.</p></div>"
                .into()
        }
        Err(Undecided::Contested {
            extension,
            claimants,
            had_rules,
        }) => {
            let names = claimants
                .iter()
                .map(|id| {
                    langbank::language_profile(id)
                        .map(|l| link(&format!("/languages/{id}"), l.display_name))
                        .unwrap_or_else(|| escape(id))
                })
                .collect::<Vec<_>>()
                .join(", ");
            let advice = if had_rules {
                "There are content rules for it — paste the file below and they will run."
            } else {
                "langbank carries no content rules for this extension, so it stays contested."
            };
            format!(
                "<div class=verdict><h3>Contested</h3><p>{} is claimed by {names}, and none of \
                 them claims it first. {advice}</p></div>",
                code(&format!(".{extension}"))
            )
        }
    }
}

pub fn render(query: &Query) -> String {
    let rules: usize = langbank::disambiguations()
        .iter()
        .map(|d| d.rules.len())
        .sum();
    let contested = langbank::disambiguations().len();

    let mut examples = String::new();
    for (path, label) in [
        ("src/main.rs", "a name that settles itself"),
        ("legacy.h", "three languages claim it"),
        ("man/git.1", "contested with no first claim"),
        ("scripts/deploy", "no extension at all"),
    ] {
        let _ = write!(
            examples,
            "<li>{} <small>{}</small></li>",
            link(&format!("/identify?path={path}"), path),
            escape(label)
        );
    }

    let body = format!(
        r##"<h1>Identify a file</h1>
<p class=lede>langbank does not read files. It carries {rules} ordered rules across
{contested} contested extensions so a caller holding the bytes can settle what a
filename cannot — and reports which rule fired, not just the verdict.</p>

{verdict}

<form method=post action="/identify">
  <label for=path>Path or filename</label>
  <input id=path name=path value="{path}" placeholder="src/main.rs" autofocus>
  <label for=content>File content <span class=none>— optional; the rules only run when there is something to read</span></label>
  <textarea id=content name=content placeholder="#!/usr/bin/env bash&#10;echo hi">{content}</textarea>
  <button type=submit>Identify</button>
</form>

<h2>Try these</h2>
<ul class=grid>{examples}</ul>

<h2>Why the evidence matters</h2>
<p>“This is C” and “this is C because rule 3 for <code>.h</code> matched” are
different claims. A consumer joining this answer to a parse tree or a compiler
observation needs the second one: it can tell a confident answer from a fallback,
and it can disagree with a specific rule rather than with langbank.</p>
<p>The same call is available without the web page:</p>
<pre><code>langbank_detect::identify("legacy.h", Some(source))
// Ok(Identification {{ language: cpp, evidence: Content {{ extension: "h", rule: 1 }} }})</code></pre>
"##,
        verdict = verdict(query),
        path = escape(&query.path),
        content = escape(&query.content),
    );
    page("Identify a file", &[("/", "langbank")], &body)
}
