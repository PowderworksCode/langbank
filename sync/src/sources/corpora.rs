//! Comment syntax and extensions from tokei and scc.
//!
//! Ported from `tools/sync-corpora.py`. Two independently maintained corpora,
//! both permissively licensed. They are genuinely independent — on the 187
//! languages both carry they agree on 77% of extension sets, 93% of line
//! comments and 89% of block comments, far from the ~100% that would mean one
//! corpus wearing two hats — so agreement between them is evidence rather than
//! an echo.
//!
//!   both carry it and agree  -> absorb, corroborated
//!   only one carries it      -> absorb, single source, as linguist already is
//!   both carry it and differ -> record the disagreement, change nothing
//!
//! A language that already has comment syntax is never touched: those entries
//! are hand-written and a corpus does not get to overrule them.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const TOKEI: &str = "https://raw.githubusercontent.com/XAMPPRocky/tokei/master/languages.json";
const SCC: &str = "https://raw.githubusercontent.com/boyter/scc/master/languages.json";
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

/// The comparable shape of a language's comment syntax.
///
/// `documentation` is deliberately outside the equality used to reconcile the
/// two corpora: only tokei publishes it, so including it would make every
/// language they both carry look like a disagreement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Syntax {
    line: Vec<String>,
    block: Vec<(String, String)>,
    documentation: Vec<String>,
}

impl Syntax {
    fn agrees_with(&self, other: &Self) -> bool {
        self.line == other.line && self.block == other.block
    }
}

fn strings(value: Option<&Value>) -> Vec<String> {
    let mut out: Vec<String> = value
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn pairs(value: Option<&Value>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = value
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|pair| {
                    let pair = pair.as_array()?;
                    if pair.len() != 2 {
                        return None;
                    }
                    Some((pair[0].as_str()?.to_string(), pair[1].as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn corpus(
    raw: &serde_json::Map<String, Value>,
    multi_key: &str,
    doc_key: Option<&str>,
) -> BTreeMap<String, Syntax> {
    let mut out = BTreeMap::new();
    for (name, entry) in raw {
        let line = strings(entry.get("line_comment"));
        let block = pairs(entry.get(multi_key));
        let documentation = doc_key.map(|k| strings(entry.get(k))).unwrap_or_default();
        if !line.is_empty() || !block.is_empty() {
            out.insert(
                slug(name),
                Syntax {
                    line,
                    block,
                    documentation,
                },
            );
        }
    }
    out
}

/// Extensions, split into unambiguous suffixes and dotted entries.
///
/// Both corpora list whole filenames alongside extensions — `cmakelists.txt`
/// sits in cmake's extension list — and langbank keeps the two apart, so
/// anything containing a dot is reported rather than filed as a suffix.
fn extensions(
    raw: &serde_json::Map<String, Value>,
    plain: &mut BTreeMap<String, BTreeSet<String>>,
    dotted: &mut BTreeMap<String, BTreeSet<String>>,
) {
    for (name, entry) in raw {
        for value in strings(entry.get("extensions")) {
            let value = value.to_lowercase();
            let value = value.trim_start_matches('.').to_string();
            let target = if value.contains('.') {
                &mut *dotted
            } else {
                &mut *plain
            };
            target.entry(slug(name)).or_default().insert(value);
        }
    }
}

struct Carried {
    path: std::path::PathBuf,
    text: String,
    has_comments: bool,
}

fn local_languages() -> Result<BTreeMap<String, Carried>> {
    let mut out = BTreeMap::new();
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        let has_comments = text.lines().any(|l| l.starts_with("comments = "));
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

fn carried_extensions(languages: &BTreeMap<String, Carried>, id: &str) -> BTreeSet<String> {
    languages
        .get(id)
        .map(|c| local::array(&c.text, "extensions").into_iter().collect())
        .unwrap_or_default()
}

/// What both corpora offer that langbank does not carry, and what they disagree
/// about — which nobody should guess at.
fn reconcile(
    tokei: &BTreeMap<String, Syntax>,
    scc: &BTreeMap<String, Syntax>,
    languages: &BTreeMap<String, Carried>,
) -> (BTreeMap<String, Syntax>, BTreeMap<String, (Syntax, Syntax)>) {
    let (mut resolved, mut conflicts) = (BTreeMap::new(), BTreeMap::new());
    for (id, carried) in languages {
        if carried.has_comments {
            continue;
        }
        match (tokei.get(id), scc.get(id)) {
            (Some(left), Some(right)) => {
                if left.agrees_with(right) {
                    resolved.insert(id.clone(), left.clone());
                } else {
                    conflicts.insert(id.clone(), (left.clone(), right.clone()));
                }
            }
            (Some(only), None) | (None, Some(only)) => {
                resolved.insert(id.clone(), only.clone());
            }
            (None, None) => {}
        }
    }
    (resolved, conflicts)
}

/// The tables already written, keyed by name, so an identical syntax reuses a
/// table rather than adding a duplicate.
fn existing_tables(text: &str) -> BTreeMap<String, Syntax> {
    let mut out = BTreeMap::new();
    for block in text.split("\n[").skip(1) {
        let Some(name) = block.split(']').next() else {
            continue;
        };
        let mut line = local::array(block, "line");
        line.sort();
        let mut documentation = local::array(block, "documentation");
        documentation.sort();
        let mut block_pairs = paired(block);
        block_pairs.sort();
        out.insert(
            name.to_string(),
            Syntax {
                line,
                block: block_pairs,
                documentation,
            },
        );
    }
    out
}

/// `["/*", "*/"]` pairs anywhere in a table body.
fn paired(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '[' {
            let rest: String = bytes[i..].iter().collect();
            if let Some(close) = rest.find(']') {
                let inner = &rest[1..close];
                let values = local::array(&format!("x = [{inner}]"), "x");
                if values.len() == 2 && inner.starts_with('"') {
                    out.push((values[0].clone(), values[1].clone()));
                }
            }
        }
        i += 1;
    }
    out
}

fn render(name: &str, syntax: &Syntax) -> String {
    let blocks = syntax
        .block
        .iter()
        .map(|(a, b)| format!("[{}, {}]", local::toml_string(a), local::toml_string(b)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "\n[{name}]\nline = {}\nblock = [{blocks}]\ndocumentation = {}\nquotes = []\nmulti-quotes = []\n",
        local::toml_array(&syntax.line),
        local::toml_array(&syntax.documentation),
    )
}

fn object(raw: &str, what: &str) -> Result<serde_json::Map<String, Value>> {
    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("{what} did not parse: {error}"))?;
    // tokei publishes `{"languages": {...}}`; scc publishes the map directly.
    let value = value
        .get("languages")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or(value);
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{what} is not an object").into())
}

pub fn run(verb: &str) -> Result<Outcome> {
    let tokei_raw = object(&fetch::text(TOKEI)?, "tokei languages.json")?;
    let scc_raw = object(&fetch::text(SCC)?, "scc languages.json")?;
    let tokei = corpus(&tokei_raw, "multi_line_comments", Some("important_syntax"));
    let scc = corpus(&scc_raw, "multi_line", None);
    let languages = local_languages()?;
    let (resolved, conflicts) = reconcile(&tokei, &scc, &languages);

    let (mut plain, mut dotted) = (BTreeMap::new(), BTreeMap::new());
    extensions(&tokei_raw, &mut plain, &mut dotted);
    extensions(&scc_raw, &mut plain, &mut dotted);

    let outstanding = |source: &BTreeMap<String, BTreeSet<String>>| {
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (id, values) in source {
            if !languages.contains_key(id) {
                continue;
            }
            let have = carried_extensions(&languages, id);
            let missing: Vec<String> = values.difference(&have).cloned().collect();
            if !missing.is_empty() {
                out.insert(id.clone(), missing);
            }
        }
        out
    };
    let missing_ext = outstanding(&plain);
    let refused = outstanding(&dotted);

    if verb == "check" {
        let have = languages.values().filter(|c| c.has_comments).count();
        println!("comment syntax: {have} of {} languages", languages.len());
        println!(
            "  available from tokei/scc and not yet carried: {}",
            resolved.len()
        );
        println!(
            "  the two corpora disagree, left alone: {}",
            conflicts.len()
        );
        for (id, (left, right)) in &conflicts {
            println!(
                "    {id}: tokei line={} block={} | scc line={} block={}",
                debug_list(&left.line),
                left.block.len(),
                debug_list(&right.line),
                right.block.len()
            );
        }
        let total: usize = missing_ext.values().map(Vec::len).sum();
        println!(
            "\nextensions not yet carried: {total} across {} languages",
            missing_ext.len()
        );
        if !refused.is_empty() {
            println!(
                "  refused as ambiguous — a dot means a filename, not a suffix: {}",
                refused.values().map(Vec::len).sum::<usize>()
            );
            for (id, values) in &refused {
                println!("    {id}: {}", values.join(" "));
            }
        }
        return Ok(Outcome::of(resolved.len() + missing_ext.len()));
    }

    let tables = existing_tables(&std::fs::read_to_string(TABLES).unwrap_or_default());
    let mut by_syntax: BTreeMap<Syntax, String> =
        tables.into_iter().map(|(name, s)| (s, name)).collect();
    let mut appended: Vec<(String, Syntax)> = Vec::new();

    for (id, syntax) in &resolved {
        let name = match by_syntax.get(syntax) {
            Some(name) => name.clone(),
            None => {
                // A table is named after the first language that needs it,
                // which is the convention the hand-written ones follow.
                by_syntax.insert(syntax.clone(), id.clone());
                appended.push((id.clone(), syntax.clone()));
                id.clone()
            }
        };
        let Some(carried) = languages.get(id) else {
            continue;
        };
        let insert = insertion_point(&carried.text);
        let text = format!(
            "{}comments = {}\n{}",
            &carried.text[..insert],
            local::toml_string(&name),
            &carried.text[insert..]
        );
        std::fs::write(&carried.path, text)
            .map_err(|e| format!("{}: {e}", carried.path.display()))?;
    }

    // A disagreement is a finding. Printing it and moving on means
    // rediscovering it on every run and never acting on it, so it is written
    // down as a gap with a reason.
    if !conflicts.is_empty() {
        let mut rows = String::new();
        for (id, (left, right)) in &conflicts {
            let note = format!(
                "tokei: line {}, {} block forms; scc: line {}, {} block forms",
                debug_list(&left.line),
                left.block.len(),
                debug_list(&right.line),
                right.block.len()
            );
            let _ = write!(
                rows,
                "\n[[gap]]\nsubject = {}\nreason = \"sources-disagree\"\nnote = {}\n",
                local::toml_string(id),
                local::toml_string(&note)
            );
        }
        std::fs::write(
            "data/gaps/comment-syntax.toml",
            format!(
                "# Comment syntax tokei and scc disagree about, so neither of them is taken.\n\
             # The disagreement is usually about block forms; a language here may still\n\
             # carry a line comment that another corroborated source settled — read its\n\
             # own entry for what langbank actually holds.\n\
             facet = \"comment-syntax\"\n{rows}"
            ),
        )
        .map_err(|e| format!("data/gaps/comment-syntax.toml: {e}"))?;
    }

    if !appended.is_empty() {
        let mut text = std::fs::read_to_string(TABLES).unwrap_or_default();
        for (name, syntax) in &appended {
            text.push_str(&render(name, syntax));
        }
        std::fs::write(TABLES, text).map_err(|e| format!("{TABLES}: {e}"))?;
    }

    // Re-read: the comment references above rewrote these files.
    let fresh = local_languages()?;
    for (id, values) in &missing_ext {
        let Some(carried) = fresh.get(id) else {
            continue;
        };
        let mut all: BTreeSet<String> = local::array(&carried.text, "extensions")
            .into_iter()
            .collect();
        all.extend(values.iter().cloned());
        let all: Vec<String> = all.into_iter().collect();
        let line = format!("extensions = {}\n", local::toml_array(&all));
        let text = match local::span(&carried.text, "extensions") {
            Some((start, end)) => {
                format!("{}{line}{}", &carried.text[..start], &carried.text[end..])
            }
            // A file with no `extensions` at all gets one directly after
            // `role`, which is where the key belongs in the canonical order —
            // not after whatever token array happens to be present. These are
            // two different anchors and using one for both reorders the file.
            None => {
                let at = after_role(&carried.text);
                format!("{}{line}{}", &carried.text[..at], &carried.text[at..])
            }
        };
        std::fs::write(&carried.path, text)
            .map_err(|e| format!("{}: {e}", carried.path.display()))?;
    }

    println!(
        "comment syntax written for {} languages, {} new shared tables, {} conflicts left alone",
        resolved.len(),
        appended.len(),
        conflicts.len()
    );
    println!(
        "extensions added: {} across {} languages; {} refused as ambiguous",
        missing_ext.values().map(Vec::len).sum::<usize>(),
        missing_ext.len(),
        refused.values().map(Vec::len).sum::<usize>()
    );
    Ok(Outcome::Complete)
}

/// Where a `comments = ` reference goes: after the first detection-token array,
/// or after `role` when the file has none.
fn insertion_point(text: &str) -> usize {
    for key in ["extensions", "filenames", "shebangs"] {
        if let Some((_, end)) = local::span(text, key) {
            return end;
        }
    }
    after_role(text)
}

/// Immediately after the `role = ` line.
fn after_role(text: &str) -> usize {
    text.find("role = ")
        .and_then(|at| text[at..].find('\n').map(|nl| at + nl + 1))
        .unwrap_or(0)
}

/// Python renders a list of strings as `['a', 'b']` in these messages, and the
/// gap notes are compared against what the script wrote, so the shape is kept.
fn debug_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
