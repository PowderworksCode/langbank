//! Language servers from nvim-lspconfig.
//!
//! Ported from `tools/sync-lspconfig.py`. A language server is a toolchain
//! entry: a program, the languages it serves, and the files it looks for to
//! decide where a project begins. Root markers are recorded on the server
//! rather than on the language, because they are the server's convention —
//! clangd wants compile_commands.json, ts_ls wants a lockfile — and
//! aggregating them per language produces noise rather than fact.
//!
//! Two exclusions, both deliberate: a server with ten or more filetypes is
//! generic tooling whose markers are its own config files and say nothing about
//! any language, and a server with no `cmd` needs a locally installed path and
//! cannot be described as a program to look for. A server that computes its
//! root imperatively is still carried, with no markers — rust_analyzer, gopls
//! and jdtls are all in that group.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use std::collections::{BTreeMap, BTreeSet};

const UPSTREAM: &str = "neovim/nvim-lspconfig";
const GENERIC_FILETYPES: usize = 10;

fn tarball() -> String {
    format!("https://codeload.github.com/{UPSTREAM}/tar.gz/refs/heads/master")
}

/// Neovim filetypes are close to langbank ids and not identical.
const ALIAS: &[(&str, &str)] = &[
    ("cs", "c-sharp"),
    ("sh", "shell"),
    ("bash", "shell"),
    ("zsh", "shell"),
    ("javascriptreact", "javascript"),
    ("typescriptreact", "typescript"),
    ("objc", "objective-c"),
    ("objcpp", "objective-cpp"),
    ("plaintex", "tex"),
    ("gomod", "go"),
    ("gowork", "go"),
    ("gotmpl", "go"),
    ("eruby", "html-erb"),
    ("make", "makefile"),
    ("yml", "yaml"),
    ("jsonc", "json-with-comments"),
    ("vb", "visual-basic-net"),
    ("ps1", "powershell"),
    ("rmd", "rmarkdown"),
    ("terraform", "hcl"),
    ("tf", "hcl"),
];

/// A Lua list literal, by brace matching.
///
/// A regex alone reads past the closing brace and swallows the next field,
/// which silently turned every filetype into a root marker once. `None` and an
/// empty list mean different things here — a server with `root_markers = {}`
/// has none, one without the key computes its root in code — so the absence is
/// preserved rather than flattened.
fn lua_list(body: &str, key: &str) -> Option<Vec<String>> {
    let pattern = regex::Regex::new(&format!(r"\b{key}\s*=\s*\{{")).ok()?;
    let found = pattern.find(body)?;
    let start = found.end() - 1;
    let mut depth = 0usize;
    for (offset, c) in body[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(single_quoted(&body[start..start + offset]));
                }
            }
            _ => {}
        }
    }
    None
}

/// `'...'` strings, honouring backslash escapes.
fn single_quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\'' {
            continue;
        }
        let mut value = String::new();
        let mut escaped = false;
        for c in chars.by_ref() {
            if escaped {
                value.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '\'' {
                break;
            } else {
                value.push(c);
            }
        }
        out.push(value);
    }
    out
}

struct Server {
    id: String,
    cmd: Vec<String>,
    filetypes: Vec<String>,
    markers: Option<Vec<String>>,
    languages: Vec<String>,
}

impl Server {
    fn toolchain_id(&self) -> String {
        format!("lsp-{}", self.id.replace('_', "-"))
    }
}

fn upstream_servers() -> Result<Vec<Server>> {
    let wanted = regex::Regex::new(r"/lsp/[^/]+\.lua$")?;
    let files = fetch::tarball(&tarball(), |name| wanted.is_match(name))?;
    let mut out = Vec::new();
    for (name, text) in files {
        // The table after `return {` is the server definition; anything above
        // it is helper code whose braces would confuse the matcher.
        let body = match text.find("\nreturn {") {
            Some(at) => &text[at..],
            None => &text,
        };
        let id = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .trim_end_matches(".lua")
            .to_string();
        out.push(Server {
            id,
            cmd: lua_list(body, "cmd").unwrap_or_default(),
            filetypes: lua_list(body, "filetypes").unwrap_or_default(),
            markers: lua_list(body, "root_markers"),
            languages: Vec::new(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn known_languages() -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(id) = local::scalar(&text, "id") {
            out.insert(id);
        }
    }
    Ok(out)
}

fn usable(
    servers: Vec<Server>,
    known: &BTreeSet<String>,
) -> (Vec<Server>, BTreeMap<&'static str, usize>) {
    let alias: BTreeMap<&str, &str> = ALIAS.iter().copied().collect();
    let to_language = |filetype: &str| -> Option<String> {
        let base = filetype.split('.').next().unwrap_or(filetype);
        let candidate = alias.get(base).copied().unwrap_or(base);
        known.contains(candidate).then(|| candidate.to_string())
    };

    let mut out = Vec::new();
    let mut dropped: BTreeMap<&'static str, usize> =
        [("generic", 0), ("unmapped", 0), ("no-command", 0)]
            .into_iter()
            .collect();
    for mut server in servers {
        if server.filetypes.len() >= GENERIC_FILETYPES {
            *dropped.entry("generic").or_default() += 1;
            continue;
        }
        let mapped: BTreeSet<String> = server
            .filetypes
            .iter()
            .filter_map(|ft| to_language(ft))
            .collect();
        if mapped.is_empty() {
            *dropped.entry("unmapped").or_default() += 1;
            continue;
        }
        if server.cmd.is_empty() {
            *dropped.entry("no-command").or_default() += 1;
            continue;
        }
        server.languages = mapped.into_iter().collect();
        out.push(server);
    }
    (out, dropped)
}

fn write(server: &Server) -> Result<()> {
    let id = server.toolchain_id();
    let mut lines = vec![
        format!("id = \"{id}\""),
        format!("display-name = {}", local::toml_string(&server.id)),
        "kind = \"language-server\"".to_string(),
        format!("languages = {}", local::toml_array(&server.languages)),
        format!("programs = {}", local::toml_array(&server.cmd[..1])),
    ];
    if let Some(markers) = &server.markers
        && !markers.is_empty()
    {
        lines.push(format!("root-markers = {}", local::toml_array(markers)));
    }
    let path = format!("data/toolchains/{id}.toml");
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{path}: {e}"))?;
    Ok(())
}

fn carried_toolchains() -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for path in local::files("data/toolchains")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(id) = local::scalar(&text, "id") {
            out.insert(id);
        }
    }
    Ok(out)
}

pub fn run(verb: &str) -> Result<Outcome> {
    let servers = upstream_servers()?;
    let total = servers.len();
    let (keep, dropped) = usable(servers, &known_languages()?);
    let have = carried_toolchains()?;
    let missing: Vec<&Server> = keep
        .iter()
        .filter(|s| !have.contains(&s.toolchain_id()))
        .collect();

    if verb == "create" {
        for server in &missing {
            write(server)?;
        }
        println!("wrote {} language servers", missing.len());
        return Ok(Outcome::Complete);
    }

    println!(
        "{total} servers upstream, {} usable, langbank carries {}",
        keep.len(),
        keep.len() - missing.len()
    );
    println!(
        "  dropped: {} generic, {} unmapped filetypes, {} without a command",
        dropped["generic"], dropped["unmapped"], dropped["no-command"]
    );
    if !missing.is_empty() {
        println!("\n{} not yet carried:", missing.len());
        for server in missing.iter().take(30) {
            println!("  {}  ({})", server.id, server.languages.join(", "));
        }
        return Ok(Outcome::Incomplete);
    }
    Ok(Outcome::Complete)
}
