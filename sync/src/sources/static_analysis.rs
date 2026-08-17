//! Linters and formatters from analysis-tools-dev/static-analysis.
//!
//! Ported from `tools/sync-static-analysis.py`. 755 tools curated per language,
//! largely disjoint from mason — 666 of them are tools langbank does not
//! otherwise know — because mason indexes what an editor can install and this
//! indexes what an analyser community has written.
//!
//! Same split as the other sources: a tool whose program langbank already knows
//! gains its categories, appended, with nothing rewritten; a tool it does not
//! know becomes a new entry. Tools categorised only as `meta` or `performance`
//! are skipped, being collections and benchmarks rather than something a
//! language can be asked through.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use std::collections::{BTreeMap, BTreeSet};

const UPSTREAM: &str = "analysis-tools-dev/static-analysis";

fn tarball() -> String {
    format!("https://codeload.github.com/{UPSTREAM}/tar.gz/refs/heads/master")
}

const CATEGORY: &[(&str, &str)] = &[("linter", "linter"), ("formatter", "formatter")];

/// The upstream tags are already close to langbank ids; these are the strays.
/// A `None` is a tag that names no language at all — `security`, `ci`, `all` —
/// and mapping it to something would invent a fact.
const ALIAS: &[(&str, Option<&str>)] = &[
    ("c++", Some("cpp")),
    ("c#", Some("c-sharp")),
    ("objective-c", Some("objective-c")),
    ("bash", Some("shell")),
    ("shell", Some("shell")),
    ("docker", Some("dockerfile")),
    ("terraform", Some("hcl")),
    ("latex", Some("tex")),
    ("golang", Some("go")),
    ("node", Some("javascript")),
    ("vue", Some("vue")),
    ("dotnet", Some("c-sharp")),
    ("protobuf", Some("protocol-buffer")),
    ("config", None),
    ("ci", None),
    ("security", None),
    ("all", None),
    ("multi", None),
];

fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.to_lowercase().chars() {
        out.push(if c.is_alphanumeric() { c } else { '-' });
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

/// A `key:` followed by an indented `- ` list, with quotes stripped.
///
/// This handles CRLF, and the script it replaces did not. Two of the 755
/// upstream files use CRLF line endings, and the script matched
/// `^categories:\n`, which cannot match `categories:\r\n` — so both parsed as
/// having no categories and no tags and were counted under "skipped (no
/// analysable category or language)". A parse failure had been sitting in the
/// bucket that means "deliberately excluded", which is why the two counts do
/// not agree with the script's: `fta` is a real linter for TypeScript and is
/// now carried. `delphilint` stays skipped, correctly — langbank has no Delphi.
fn block(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line == format!("{key}:") {
            inside = true;
            continue;
        }
        if inside {
            match line.strip_prefix("  - ") {
                Some(value) => out.push(value.trim().trim_matches(['\'', '"']).to_string()),
                None => break,
            }
        }
    }
    out
}

struct Tool {
    name: String,
    categories: Vec<String>,
    tags: Vec<String>,
    homepage: Option<String>,
    repository: Option<String>,
}

fn upstream_tools() -> Result<Vec<Tool>> {
    let wanted = regex::Regex::new(r"/data/tools/[^/]+\.yml$")?;
    let files = fetch::tarball(&tarball(), |name| wanted.is_match(name))?;
    let mut out = Vec::new();
    for (_, text) in files {
        let Some(name) = text.lines().find_map(|line| {
            line.strip_prefix("name:")
                .map(|v| v.trim().trim_matches(['\'', '"']).to_string())
        }) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        // `homepage` is dropped when it is the repository under another
        // spelling — 368 of the 656 that publish both publish the same URL
        // twice, and recording it twice makes a reader look for a difference
        // that is not there. See `langbank::Origin`.
        let repository = scalar(&text, "source");
        let homepage = scalar(&text, "homepage").filter(|home| {
            repository
                .as_deref()
                .is_none_or(|repo| !langbank::same_place(home, repo))
        });
        out.push(Tool {
            name,
            categories: block(&text, "categories"),
            tags: block(&text, "tags"),
            homepage,
            repository,
        });
    }
    out.sort_by_key(|tool| tool.name.to_lowercase());
    Ok(out)
}

struct Local {
    by_display: BTreeMap<String, String>,
    toolchains: BTreeMap<String, (std::path::PathBuf, String)>,
    programs: BTreeMap<String, String>,
}

fn langbank() -> Result<Local> {
    let mut by_display = BTreeMap::new();
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        by_display.insert(id.clone(), id.clone());
        if let Some(display) = local::scalar(&text, "display-name") {
            by_display.entry(display.to_lowercase()).or_insert(id);
        }
    }

    let (mut toolchains, mut programs) = (BTreeMap::new(), BTreeMap::new());
    for path in local::files("data/toolchains")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        for program in local::array(&text, "programs") {
            programs
                .entry(program.to_lowercase())
                .or_insert_with(|| id.clone());
        }
        if let Some(display) = local::scalar(&text, "display-name") {
            programs
                .entry(display.to_lowercase())
                .or_insert_with(|| id.clone());
        }
        toolchains.insert(id, (path, text));
    }
    Ok(Local {
        by_display,
        toolchains,
        programs,
    })
}

struct Entry {
    id: String,
    name: String,
    kinds: Vec<String>,
    languages: Vec<String>,
    homepage: Option<String>,
    repository: Option<String>,
}

impl Entry {
    /// The same, judged against an existing entry's `kind` when there is one:
    /// a merge must not repeat the kind of the file it is writing into, which
    /// is not necessarily this source's own primary.
    fn others_besides(&self, kind: Option<&str>) -> Vec<String> {
        let Some(primary) = kind.or_else(|| self.kinds.first().map(String::as_str)) else {
            return Vec::new();
        };
        self.kinds
            .iter()
            .filter(|other| other.as_str() != primary)
            .cloned()
            .collect()
    }
}

/// The `key: value` lines that are not lists.
fn scalar(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
        let value = value.trim_matches(['\'', '"']);
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// The lines a merge would append: only what the file does not already carry.
fn additions(entry: &Entry, text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    // `categories` records what a tool does *besides* its kind, so a list that
    // only restates the kind is not written. 906 entries carried one, and `is`
    // answered every one of them from `kind` alone.
    let others = entry.others_besides(local::scalar(text, "kind").as_deref());
    if !others.is_empty() && !text.contains("categories") {
        lines.push(format!("categories = {}", local::toml_array(&others)));
    }
    if let Some(homepage) = &entry.homepage
        && !text.contains("homepage")
    {
        lines.push(format!("homepage = {}", local::toml_string(homepage)));
    }
    if let Some(repository) = &entry.repository
        && !text.contains("repository")
    {
        lines.push(format!("repository = {}", local::toml_string(repository)));
    }
    lines
}

fn plan(tools: &[Tool], carried: &Local) -> (Vec<(String, Entry)>, Vec<Entry>, usize) {
    let category: BTreeMap<&str, &str> = CATEGORY.iter().copied().collect();
    let alias: BTreeMap<&str, Option<&str>> = ALIAS.iter().copied().collect();
    let (mut merges, mut creates, mut skipped) = (Vec::new(), Vec::new(), 0usize);

    for tool in tools {
        let kinds: Vec<String> = tool
            .categories
            .iter()
            .filter_map(|c| category.get(c.as_str()).map(|k| k.to_string()))
            .collect();
        if kinds.is_empty() {
            skipped += 1;
            continue;
        }
        let languages: BTreeSet<String> = tool
            .tags
            .iter()
            .filter_map(|tag| {
                let key = tag.to_lowercase();
                let mapped = match alias.get(key.as_str()) {
                    Some(None) => return None,
                    Some(Some(mapped)) => (*mapped).to_string(),
                    None => key,
                };
                carried.by_display.get(&mapped).cloned()
            })
            .collect();
        if languages.is_empty() {
            skipped += 1;
            continue;
        }
        let entry = Entry {
            id: format!("sa-{}", slug(&tool.name)),
            name: tool.name.clone(),
            kinds,
            languages: languages.into_iter().collect(),
            homepage: tool.homepage.clone(),
            repository: tool.repository.clone(),
        };
        match carried.programs.get(&tool.name.to_lowercase()) {
            Some(existing) => {
                let Some((_, text)) = carried.toolchains.get(existing) else {
                    continue;
                };
                // Outstanding when there is anything to add, not only when
                // categories are missing — otherwise a tool that already has
                // categories never gains its links.
                if !additions(&entry, text).is_empty() {
                    merges.push((existing.clone(), entry));
                }
            }
            None => {
                if !carried.toolchains.contains_key(&entry.id) {
                    creates.push(entry);
                }
            }
        }
    }
    (merges, creates, skipped)
}

pub fn run(verb: &str) -> Result<Outcome> {
    let tools = upstream_tools()?;
    let carried = langbank()?;
    let (merges, creates, skipped) = plan(&tools, &carried);

    if verb == "check" {
        println!(
            "{} tools upstream; {} would gain categories or links, {} are new, {skipped} skipped \
             (no analysable category or language)",
            tools.len(),
            merges.len(),
            creates.len()
        );
        return Ok(Outcome::of(merges.len() + creates.len()));
    }

    for (id, entry) in &merges {
        let Some((path, text)) = carried.toolchains.get(id) else {
            continue;
        };
        let lines = additions(entry, text);
        if lines.is_empty() {
            continue;
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        // Root keys go before the first [table] header, not at the end of the
        // file — see `local::append_root`.
        std::fs::write(path, local::append_root(&text, &lines))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }

    for entry in &creates {
        let lines = [
            format!("id = \"{}\"", entry.id),
            format!("display-name = {}", local::toml_string(&entry.name)),
            format!("kind = \"{}\"", entry.kinds[0]),
            format!("languages = {}", local::toml_array(&entry.languages)),
            format!(
                "programs = {}",
                local::toml_array(&[entry.name.to_lowercase()])
            ),
        ]
        .into_iter()
        .chain(
            entry
                .homepage
                .iter()
                .map(|home| format!("homepage = {}", local::toml_string(home))),
        )
        .chain(
            entry
                .repository
                .iter()
                .map(|repo| format!("repository = {}", local::toml_string(repo))),
        )
        .collect::<Vec<_>>();
        let path = format!("data/toolchains/{}.toml", entry.id);
        std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{path}: {e}"))?;
    }

    println!(
        "merged into {} known tools, created {} new, {skipped} skipped",
        merges.len(),
        creates.len()
    );
    Ok(Outcome::Complete)
}
