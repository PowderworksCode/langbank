//! Tool distribution from mason-registry.
//!
//! Ported from `tools/sync-mason.py`. Mason is the inverse index of
//! nvim-lspconfig: lspconfig knows how to *run* a tool, mason knows what it *is*
//! and how it is *distributed* — and it says so in purl, the vocabulary
//! `data/registries/` already carries.
//!
//! Two jobs kept apart, as the other sync sources do: a package whose program
//! langbank already knows gains its distribution and categories, appended, with
//! nothing existing rewritten; a package langbank does not know becomes a new
//! toolchain entry. Mason names languages in prose — `LaTeX`, `Bash` — so those
//! are mapped, and a package whose languages map to nothing is skipped rather
//! than filed against an invented id.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use std::collections::{BTreeMap, BTreeSet};

const UPSTREAM: &str = "mason-org/mason-registry";

fn tarball() -> String {
    format!("https://codeload.github.com/{UPSTREAM}/tar.gz/refs/heads/main")
}

const CATEGORY: &[(&str, &str)] = &[
    ("LSP", "language-server"),
    ("Formatter", "formatter"),
    ("Linter", "linter"),
    ("DAP", "debugger"),
    ("Runtime", "runtime"),
    ("Compiler", "compiler"),
];

/// Mason writes language names for people to read.
const ALIAS: &[(&str, &str)] = &[
    ("bash", "shell"),
    ("sh", "shell"),
    ("latex", "tex"),
    ("terraform", "hcl"),
    ("docker", "dockerfile"),
    ("c#", "c-sharp"),
    ("c++", "cpp"),
    ("f#", "f-sharp"),
    ("objective-c", "objective-c"),
    ("golang", "go"),
    ("protobuf", "protocol-buffer"),
    ("javascript react", "javascript"),
    ("typescript react", "typescript"),
];

/// A `key:` followed by an indented `- ` list.
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
                Some(value) => out.push(value.to_string()),
                None => break,
            }
        }
    }
    out
}

/// A bare `key: value`, at any indentation.
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix(key)?.strip_prefix(':')?;
        let value = rest.trim();
        (!value.is_empty()).then(|| value.split_whitespace().next().unwrap_or(value).to_string())
    })
}

struct Package {
    name: String,
    languages: Vec<String>,
    categories: Vec<String>,
    purl: Option<String>,
    homepage: Option<String>,
}

fn upstream_packages() -> Result<Vec<Package>> {
    let files = fetch::tarball(&tarball(), |name| name.ends_with("/package.yaml"))?;
    let mut out = Vec::new();
    for (_, text) in files {
        let Some(name) = field(&text, "name") else {
            continue;
        };
        out.push(Package {
            name,
            languages: block(&text, "languages"),
            categories: block(&text, "categories"),
            purl: field(&text, "id"),
            homepage: field(&text, "homepage"),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Split `pkg:type/namespace/name@version` into its type and its name.
///
/// The version is after the *last* `@`, not the first. An npm scope is part of
/// the name and is spelled either `@scope/name` or percent-encoded as
/// `%40scope/name`, so a pattern that stops at the first `@` drops every scoped
/// package — which is what the first version did, losing the distribution for
/// thirteen tools without saying anything.
fn purl_parts(purl: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(purl) = purl else {
        return (None, None);
    };
    let Some(rest) = purl.strip_prefix("pkg:") else {
        return (None, None);
    };
    let Some((kind, rest)) = rest.split_once('/') else {
        return (None, None);
    };
    if kind.is_empty()
        || !kind
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
    {
        return (None, None);
    }
    let rest = rest.split('?').next().unwrap_or(rest);
    let rest = rest.split('#').next().unwrap_or(rest);
    let rest = match rest.rfind('@') {
        Some(at) if at > 0 => &rest[..at],
        _ => rest,
    };
    (Some(kind.to_string()), Some(percent_decode(rest)))
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
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
        let display = local::scalar(&text, "display-name").unwrap_or_else(|| id.clone());
        by_display.entry(display.to_lowercase()).or_insert(id);
    }

    let (mut toolchains, mut programs) = (BTreeMap::new(), BTreeMap::new());
    for path in local::files("data/toolchains")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        for program in local::array(&text, "programs") {
            programs.entry(program).or_insert_with(|| id.clone());
        }
        // lspconfig names a server by its own id; mason names the same tool by
        // its package. Matching on both is what stops one tool becoming two.
        if let Some(display) = local::scalar(&text, "display-name") {
            programs.entry(display).or_insert_with(|| id.clone());
        }
        toolchains.insert(id, (path, text));
    }
    Ok(Local {
        by_display,
        toolchains,
        programs,
    })
}

fn to_language(name: &str, by_display: &BTreeMap<String, String>) -> Option<String> {
    let alias: BTreeMap<&str, &str> = ALIAS.iter().copied().collect();
    let key = name.to_lowercase();
    let mapped = alias.get(key.as_str()).copied().unwrap_or(key.as_str());
    by_display
        .get(mapped)
        .or_else(|| by_display.get(&key))
        .cloned()
}

struct Entry {
    name: String,
    mapped: Vec<String>,
    kinds: Vec<String>,
    registry: Option<String>,
    package: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
}

impl Entry {
    /// The same, judged against an existing entry's `kind` when there is one.
    ///
    /// A merge writes into a file that already has a kind, and that is the kind
    /// the new categories must not repeat. Judging against mason's own primary
    /// instead wrote `categories = ["language-server"]` onto `lsp-solidity`,
    /// whose kind is `language-server`: mason's `solidity` package is
    /// `[Compiler, LSP]`, so "not the primary" meant something different at
    /// each end.
    fn others_besides(&self, kind: Option<&str>) -> Vec<String> {
        let primary = kind
            .or_else(|| self.kinds.first().map(String::as_str))
            .unwrap_or("linter");
        self.kinds
            .iter()
            .filter(|other| other.as_str() != primary)
            .cloned()
            .collect()
    }
}

/// A `pkg:github/owner/name` purl names a repository; nothing else here does.
/// A crates.io or npm package has a registry page, which is where it is
/// distributed rather than where its code is, so it is not a repository.
fn repository_of(registry: Option<&str>, package: Option<&str>) -> Option<String> {
    match (registry, package) {
        (Some("github"), Some(package)) => Some(format!("https://github.com/{package}")),
        _ => None,
    }
}

/// The lines a merge would append: only what the file does not already carry.
fn additions(entry: &Entry, text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    // Only what the tool does besides its kind — see `Toolchain::categories`.
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
    if let (Some(registry), Some(package)) = (&entry.registry, &entry.package)
        && !text.contains("[distribution]")
    {
        lines.push(format!(
            "\n[distribution]\nregistry = {}",
            local::toml_string(registry)
        ));
        lines.push(format!("package = {}", local::toml_string(package)));
    }
    lines
}

/// Entries whose upstream match is recorded as ambiguous.
///
/// Removing a wrong merge by hand is worthless if the next `create` puts it
/// back, so the gap is consulted rather than being a note for a reader.
/// `lsp-solidity` is the case: mason's `solidity` is the ethereum compiler and
/// langbank's entry is a language server that shares its display name.
fn disputed() -> Vec<&'static str> {
    langbank::gaps()
        .iter()
        .filter(|gap| gap.facet == "toolchain-identity")
        .map(|gap| gap.subject)
        .collect()
}

pub fn run(verb: &str) -> Result<Outcome> {
    let packages = upstream_packages()?;
    let carried = langbank()?;
    let disputed = disputed();
    let category: BTreeMap<&str, &str> = CATEGORY.iter().copied().collect();
    let mut registries = BTreeSet::new();
    for path in local::files("data/registries")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(id) = local::scalar(&text, "id") {
            registries.insert(id);
        }
    }

    let (mut merges, mut creates, mut skipped) = (Vec::new(), Vec::new(), 0usize);
    for package in &packages {
        let mapped: BTreeSet<String> = package
            .languages
            .iter()
            .filter_map(|name| to_language(name, &carried.by_display))
            .collect();
        if mapped.is_empty() {
            skipped += 1;
            continue;
        }
        let kinds: Vec<String> = package
            .categories
            .iter()
            .filter_map(|c| category.get(c.as_str()).map(|k| k.to_string()))
            .collect();
        let (registry, name) = purl_parts(package.purl.as_deref());
        let repository = repository_of(registry.as_deref(), name.as_deref());
        // As in static-analysis: a homepage that is the repository under
        // another spelling is not a second fact. See `langbank::Origin`.
        let homepage = package.homepage.clone().filter(|home| {
            repository
                .as_deref()
                .is_none_or(|repo| !langbank::same_place(home, repo))
        });
        let entry = Entry {
            name: package.name.clone(),
            mapped: mapped.into_iter().collect(),
            kinds,
            registry,
            package: name,
            homepage,
            repository,
        };
        match carried.programs.get(&package.name) {
            Some(existing) => {
                if disputed.contains(&existing.as_str()) {
                    continue;
                }
                let Some((_, text)) = carried.toolchains.get(existing) else {
                    continue;
                };
                // Outstanding only when there is something to write. A package
                // with no purl gets no distribution block, so counting it as
                // pending would leave the check permanently red — and a check
                // that cannot go green teaches everyone to ignore it.
                if !additions(&entry, text).is_empty() {
                    merges.push((existing.clone(), entry));
                }
            }
            None => {
                if !carried
                    .toolchains
                    .contains_key(&format!("mason-{}", package.name))
                {
                    creates.push(entry);
                }
            }
        }
    }

    if verb == "check" {
        println!(
            "{} mason packages; {} would gain links or distribution, {} are new, \
             {skipped} skipped for unmapped languages",
            packages.len(),
            merges.len(),
            creates.len()
        );
        let unknown: BTreeSet<&String> = merges
            .iter()
            .map(|(_, entry)| entry)
            .chain(creates.iter())
            .filter_map(|entry| entry.registry.as_ref())
            .filter(|registry| !registries.contains(*registry))
            .collect();
        if !unknown.is_empty() {
            let shown: Vec<String> = unknown.iter().map(|r| format!("'{r}'")).collect();
            println!(
                "  purl types mason uses that purl does not define: [{}]",
                shown.join(", ")
            );
        }
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
        let kind = entry.kinds.first().map(String::as_str).unwrap_or("linter");
        let mut lines = vec![
            format!("id = \"mason-{}\"", entry.name),
            format!("display-name = {}", local::toml_string(&entry.name)),
            format!("kind = \"{kind}\""),
            format!("languages = {}", local::toml_array(&entry.mapped)),
            format!(
                "programs = {}",
                local::toml_array(std::slice::from_ref(&entry.name))
            ),
        ];
        let others = entry.others_besides(None);
        if !others.is_empty() {
            lines.push(format!("categories = {}", local::toml_array(&others)));
        }
        if let Some(homepage) = &entry.homepage {
            lines.push(format!("homepage = {}", local::toml_string(homepage)));
        }
        if let Some(repository) = &entry.repository {
            lines.push(format!("repository = {}", local::toml_string(repository)));
        }
        if let (Some(registry), Some(package)) = (&entry.registry, &entry.package) {
            lines.push(format!(
                "\n[distribution]\nregistry = {}",
                local::toml_string(registry)
            ));
            lines.push(format!("package = {}", local::toml_string(package)));
        }
        let path = format!("data/toolchains/mason-{}.toml", entry.name);
        std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{path}: {e}"))?;
    }

    println!(
        "merged distribution into {} known tools, created {} new, {skipped} skipped for \
         unmapped languages",
        merges.len(),
        creates.len()
    );
    Ok(Outcome::Complete)
}
