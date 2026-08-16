//! The introduction. Every number on it is counted from the registry at render
//! time, so the page cannot claim a coverage the data does not have.

use crate::render::page;
use maud::{Markup, html};

fn stat(href: &str, n: usize, label: &str) -> Markup {
    html! { a.stat href=(href) { b { (n) } span { (label) } } }
}

pub fn render() -> String {
    let languages = langbank::language_profiles().len();
    let rules: usize = langbank::disambiguations()
        .iter()
        .map(|d| d.rules.len())
        .sum();
    let gaps = langbank::gaps().len();

    let body = html! {
        h1 { "What every tool has to look up, in one place" }
        p.lede {
            "Which languages exist, what files they claim, which ecosystems and
             toolchains build them, and how package identity is spelled in each
             registry. langbank carries the answers as data and depends on nothing
             to do it."
        }

        div.stats {
            (stat("/languages", languages, "languages"))
            (stat("/ecosystems", langbank::ecosystem_profiles().len(), "ecosystems"))
            (stat("/toolchains", langbank::toolchains().len(), "toolchains"))
            (stat("/registries", langbank::package_registries().len(), "package registries"))
            (stat("/tools", langbank::tool_profiles().len(), "tool profiles"))
            (stat("/identify", rules, "content rules"))
        }

        h2 { "Compiled, not parsed" }
        p {
            "The data lives as TOML in the repository and a build script turns it
             into " code { "&'static" } " tables. Nothing is read at run time, so a
             consumer pays no startup cost, cannot fail to find a data directory,
             and cannot be handed a different answer by a file on disk. This page is
             rendered from the same statics any other consumer gets."
        }

        pre { code {
"langbank::language_profile(\"rust\")
    .map(|l| l.extensions)          // [\"rs\"]

langbank::detect_language(Path::new(\"src/main.rs\"), None)
    .map(|d| d.language.id)         // \"rust\""
        } }

        h2 { "It answers, or it says why not" }
        p {
            "A registry that quietly omits what it does not know is
             indistinguishable from one that has never been asked. langbank records "
            a href="/gaps" { (gaps) " absences with a reason" }
            " — sources disagreed, only one source said it, deliberately excluded,
             not modelled yet — so a consumer can tell “no” from “unknown”."
        }

        h2 { "Rules it carries but does not run" }
        p {
            "Some extensions are claimed by several languages, and no amount of
             looking at a filename settles them. langbank carries " (rules) " ordered
             rules that read the bytes instead, and reports which one fired rather
             than only the verdict. " a href="/identify" { "Try it" } " — the rules
             run in this process, against the same data the crate ships."
        }

        h2 { "A leaf, on purpose" }
        p {
            "langbank has no dependencies and will not grow any. It is the bottom of
             a stack: " b { "entl" } " walks codebases and forges, " b { "treebank" }
             " keeps tree-sitter grammars, " b { "propbank" } " marshals compiler
             facts. Each of them needs to agree on what a language is, and none of
             them should have to own that answer."
        }
    };
    page("Languages, ecosystems and toolchains as data", &[], body)
}
