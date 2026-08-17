//! Line comments, where Pygments and highlight.js independently agree.
//!
//! tokei and scc are exhausted: between them they cover the languages anybody
//! counts lines of, and 380 of langbank's 557 programming languages still have
//! no comment syntax. This looks for more, and the interesting part is what it
//! refuses.
//!
//! Neither source *states* a comment marker. Pygments encodes it in a lexer
//! regex, highlight.js in a named mode or a `COMMENT(...)` call. Reading either
//! means inferring a fact from an implementation, and an inference is not a
//! source — so the two are read separately and only their agreement is taken.
//!
//! Measured before writing any of this: the Pygments extraction agrees with
//! langbank's existing entries on 70 of 72 comparable languages. The two misses
//! are artefacts of the extraction rather than errors in Pygments — `luau`
//! picks up a shebang, `mako` a template delimiter — which is exactly why a
//! 97%-accurate inference is not absorbed on its own.
//!
//! Two sources considered and rejected:
//!
//!   * **VS Code** `language-configuration.json` has the right shape and the
//!     wrong meaning. It describes what the editor inserts when you toggle a
//!     comment, so it claims `json` has `//`. Of five entries langbank lacked,
//!     three were wrong.
//!   * **neovim** ships `commentstring` for 438 filetypes under a licence
//!     GitHub cannot identify, which is not something to absorb.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use std::collections::{BTreeMap, BTreeSet};

const PYGMENTS: &str = "https://codeload.github.com/pygments/pygments/tar.gz/refs/heads/master";
const HIGHLIGHT: &str =
    "https://codeload.github.com/highlightjs/highlight.js/tar.gz/refs/heads/main";
const TABLES: &str = "data/comment-syntax.toml";

fn slug(name: &str) -> String {
    let lowered = name
        .to_lowercase()
        .replace('#', "-sharp")
        .replace("++", "pp")
        .replace('*', "-star");
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        out.push(if c.is_alphanumeric() { c } else { '-' });
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

/// A marker worth believing: short, literal, no regex metacharacters.
fn literal(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 4
        && !token.chars().any(|c| {
            matches!(
                c,
                '\\' | '[' | ']' | '(' | ')' | '{' | '}' | '|' | '+' | '*' | '?' | '^' | '$'
            )
        })
}

/// Pygments states a lexer's human name and its comment tokens as regexes.
fn pygments() -> Result<BTreeMap<String, BTreeSet<String>>> {
    let files = fetch::tarball(PYGMENTS, |name| {
        name.contains("/pygments/lexers/") && name.ends_with(".py")
    })?;
    // Split on the class boundary rather than matching up to a lookahead:
    // this crate's regex has no look-around, which is the same constraint
    // langbank records against linguist's rules.
    let name = regex::Regex::new(r#"(?m)^\s*name\s*=\s*['"]([^'"]+)['"]"#)?;
    let token = regex::Regex::new(
        r#"\(\s*r?['"]([^'"]{1,12}?)['"]\s*,\s*(?:bygroups\([^)]*\)|Comment\.Single|Comment\.Singleline|Comment)\b"#,
    )?;
    let tail = regex::Regex::new(r"\.\*|\[\^")?;

    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, text) in files {
        for body in text.split("\nclass ").skip(1) {
            let Some(label) = name.captures(body).and_then(|c| c.get(1)) else {
                continue;
            };
            let markers: BTreeSet<String> = token
                .captures_iter(body)
                .filter_map(|c| c.get(1))
                .map(|m| tail.split(m.as_str()).next().unwrap_or("").to_string())
                .filter(|token| literal(token))
                .collect();
            if !markers.is_empty() {
                out.entry(slug(label.as_str())).or_default().extend(markers);
            }
        }
    }
    Ok(out)
}

/// highlight.js names its common comment modes, which read far more cleanly
/// than a regex: `C_LINE_COMMENT_MODE` is `//` and says so.
fn highlight() -> Result<BTreeMap<String, BTreeSet<String>>> {
    const NAMED: &[(&str, &str)] = &[
        ("C_LINE_COMMENT_MODE", "//"),
        ("SLASH_SLASH_COMMENT_MODE", "//"),
        ("HASH_COMMENT_MODE", "#"),
        ("NUMBER_SIGN_COMMENT_MODE", "#"),
        ("APOS_COMMENT_MODE", "'"),
        ("QUOTE_COMMENT_MODE", "\""),
    ];
    let files = fetch::tarball(HIGHLIGHT, |name| {
        name.contains("/src/languages/") && name.ends_with(".js")
    })?;
    let label = regex::Regex::new(r#"name:\s*['"]([^'"]+)['"]"#)?;
    let explicit = regex::Regex::new(r#"COMMENT\(\s*(?:/)?['"]([^'"]{1,6}?)['"]"#)?;
    let slashes = regex::Regex::new(r"\\+")?;

    let mut out = BTreeMap::new();
    for (path, text) in files {
        let mut markers = BTreeSet::new();
        for (mode, marker) in NAMED {
            if text.contains(mode) {
                markers.insert((*marker).to_string());
            }
        }
        for capture in explicit.captures_iter(&text) {
            let Some(found) = capture.get(1) else {
                continue;
            };
            let token = slashes.replace_all(found.as_str(), "").to_string();
            if literal(&token) {
                markers.insert(token);
            }
        }
        if markers.is_empty() {
            continue;
        }
        let fallback = path
            .rsplit('/')
            .next()
            .unwrap_or(&path)
            .trim_end_matches(".js")
            .to_string();
        let named = label
            .captures(&text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or(fallback);
        out.insert(slug(&named), markers);
    }
    Ok(out)
}

struct Carried {
    path: std::path::PathBuf,
    text: String,
    has_comments: bool,
}

fn languages() -> Result<BTreeMap<String, Carried>> {
    let mut out = BTreeMap::new();
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        let has_comments = text.lines().any(|line| line.starts_with("comments = "));
        out.insert(
            id,
            Carried {
                path,
                has_comments,
                text,
            },
        );
    }
    Ok(out)
}

/// Where both sources name the same marker, for a language langbank carries
/// and has no comment syntax for.
fn corroborated(
    left: &BTreeMap<String, BTreeSet<String>>,
    right: &BTreeMap<String, BTreeSet<String>>,
    carried: &BTreeMap<String, Carried>,
) -> (BTreeMap<String, Vec<String>>, usize, usize) {
    let (mut agreed, mut disputed, mut single) = (BTreeMap::new(), 0, 0);
    for (id, ours) in left {
        if !carried.contains_key(id) {
            continue;
        }
        match right.get(id) {
            Some(theirs) => {
                let shared: Vec<String> = ours.intersection(theirs).cloned().collect();
                if shared.is_empty() {
                    disputed += 1;
                } else if !carried.get(id).is_some_and(|entry| entry.has_comments) {
                    agreed.insert(id.clone(), shared);
                }
            }
            None => single += 1,
        }
    }
    single += right
        .keys()
        .filter(|id| carried.contains_key(*id) && !left.contains_key(*id))
        .count();
    (agreed, disputed, single)
}

/// The name of a table whose `line` is exactly these markers, if one exists.
fn table_named(tables: &str, markers: &[String]) -> Option<String> {
    for block in tables.split("\n[").skip(1) {
        let name = block.split(']').next()?;
        let mut line = local::array(block, "line");
        line.sort();
        let block_forms = local::array(block, "block");
        let documentation = local::array(block, "documentation");
        let mut wanted = markers.to_vec();
        wanted.sort();
        if line == wanted && block_forms.is_empty() && documentation.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

pub fn run(verb: &str) -> Result<Outcome> {
    let left = pygments()?;
    let right = highlight()?;
    let carried = languages()?;
    let (agreed, disputed, single) = corroborated(&left, &right, &carried);

    if verb == "check" {
        println!(
            "pygments describes {} languages, highlight.js {}",
            left.len(),
            right.len()
        );
        println!("  both agree and langbank lacks it : {}", agreed.len());
        println!("  the two disagree, left alone     : {disputed}");
        println!("  only one source names it, refused: {single}");
        for (id, markers) in agreed.iter().take(20) {
            println!("    {id}: {}", markers.join(" "));
        }
        return Ok(Outcome::of(agreed.len()));
    }

    let mut tables = std::fs::read_to_string(TABLES).unwrap_or_default();
    let mut written = 0;
    for (id, markers) in &agreed {
        let Some(entry) = carried.get(id) else {
            continue;
        };
        let name = match table_named(&tables, markers) {
            Some(name) => name,
            None => {
                // A table is named after the first language that needs it,
                // which is what the hand-written ones already do.
                tables.push_str(&format!(
                    "\n[{id}]\nline = {}\nblock = []\ndocumentation = []\nquotes = []\nmulti-quotes = []\n",
                    local::toml_array(markers)
                ));
                id.clone()
            }
        };
        let insert = entry
            .text
            .find("role = ")
            .and_then(|at| entry.text[at..].find('\n').map(|nl| at + nl + 1))
            .unwrap_or(0);
        let text = format!(
            "{}comments = {}\n{}",
            &entry.text[..insert],
            local::toml_string(&name),
            &entry.text[insert..]
        );
        std::fs::write(&entry.path, text).map_err(|e| format!("{}: {e}", entry.path.display()))?;
        written += 1;
    }
    std::fs::write(TABLES, tables).map_err(|e| format!("{TABLES}: {e}"))?;
    println!("comment syntax written for {written} languages, corroborated by both sources");
    Ok(Outcome::Complete)
}
