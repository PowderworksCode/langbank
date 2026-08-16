//! The introduction. Every number on it is counted from the registry at render
//! time, so the page cannot claim a coverage the data does not have.

use crate::render::{escape, page};
use std::fmt::Write as _;

fn stat(href: &str, n: usize, label: &str) -> String {
    format!(
        "<a class=stat href=\"{}\"><b>{n}</b><span>{}</span></a>",
        escape(href),
        escape(label)
    )
}

pub fn render() -> String {
    let languages = langbank::language_profiles().len();
    let ecosystems = langbank::ecosystem_profiles().len();
    let toolchains = langbank::toolchains().len();
    let registries = langbank::package_registries().len();
    let tools = langbank::tool_profiles().len();
    let rules: usize = langbank::disambiguations()
        .iter()
        .map(|d| d.rules.len())
        .sum();
    let gaps = langbank::gaps().len();

    let mut stats = String::new();
    for (href, n, label) in [
        ("/languages", languages, "languages"),
        ("/ecosystems", ecosystems, "ecosystems"),
        ("/toolchains", toolchains, "toolchains"),
        ("/registries", registries, "package registries"),
        ("/tools", tools, "tool profiles"),
        ("/identify", rules, "content rules"),
    ] {
        let _ = write!(stats, "{}", stat(href, n, label));
    }

    let body = format!(
        r#"<h1>What every tool has to look up, in one place</h1>
<p class=lede>Which languages exist, what files they claim, which ecosystems and
toolchains build them, and how package identity is spelled in each registry.
langbank carries the answers as data and depends on nothing to do it.</p>

<div class=stats>{stats}</div>

<h2>Compiled, not parsed</h2>
<p>The data lives as TOML in the repository and a build script turns it into
<code>&amp;'static</code> tables. Nothing is read at run time, so a consumer pays
no startup cost, cannot fail to find a data directory, and cannot be handed a
different answer by a file on disk. This page is rendered from the same statics
any other consumer gets.</p>

<pre><code>langbank::language_profile("rust")
    .map(|l| l.extensions)          // ["rs"]

langbank::detect_language(Path::new("src/main.rs"), None)
    .map(|d| d.language.id)         // "rust"</code></pre>

<h2>It answers, or it says why not</h2>
<p>A registry that quietly omits what it does not know is indistinguishable from
one that has never been asked. langbank records <a href="/gaps">{gaps} absences with a
reason</a> — sources disagreed, only one source said it, deliberately excluded,
not modelled yet — so a consumer can tell "no" from "unknown".</p>

<h2>Rules it carries but does not run</h2>
<p>Some extensions are claimed by several languages, and no amount of looking at
a filename settles them. langbank carries {rules} ordered rules that read the
bytes instead, and reports which one fired rather than only the verdict.
<a href="/identify">Try it</a> — the rules run in this process, against the same
data the crate ships.</p>

<h2>A leaf, on purpose</h2>
<p>langbank has no dependencies and will not grow any. It is the bottom of a
stack: <b>entl</b> walks codebases and forges, <b>treebank</b> keeps tree-sitter
grammars, <b>propbank</b> marshals compiler facts. Each of them needs to agree on
what a language is, and none of them should have to own that answer.</p>
"#
    );
    page("Languages, ecosystems and toolchains as data", &[], &body)
}
