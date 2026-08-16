//! The introduction, which is mostly numbers.
//!
//! An earlier version explained langbank in five sections of prose and left a
//! reader no better able to say what was actually in it. What is worth seeing
//! first is the shape of what is known and what is not — so the distribution
//! is above the fold and the argument is two sentences.

use crate::render::{bar, link, page};
use langbank::{Facet, coverage, distribution};
use maud::{Markup, html};

fn stat(href: &str, n: usize, label: &str) -> Markup {
    html! { a.stat href=(href) { b { (n) } span { (label) } } }
}

pub fn render() -> String {
    let languages = langbank::language_profiles().len();
    let carried = coverage();
    let spread = distribution();
    let widest = spread.iter().copied().max().unwrap_or(1);

    let body = html! {
        h1 { "What every tool has to look up" }
        p.lede {
            "Which languages exist, what files they claim, what builds them, and how
             package identity is spelled in each registry — as data, in a crate that
             depends on nothing."
        }

        div.stats {
            (stat("/languages", languages, "languages"))
            (stat("/ecosystems", langbank::ecosystem_profiles().len(), "ecosystems"))
            (stat("/toolchains", langbank::toolchains().len(), "toolchains"))
            (stat("/registries", langbank::package_registries().len(), "registries"))
            (stat("/gaps", langbank::gaps().len(), "recorded gaps"))
        }

        h2 { "What is actually known" }
        p {
            "Eight things langbank can know about a language. Most know one. "
            (link("/coverage", "Which, and which languages are thin."))
        }

        div.scroll { table.facets {
            thead { tr { th { "facet" } th { "lets a consumer" } th.num { "languages" } th {} } }
            tbody {
                @for (facet, have) in Facet::ALL.into_iter().zip(carried) {
                    tr {
                        td { code { (facet.name()) } }
                        td.dim { (facet.purpose()) }
                        td.num { (have) }
                        td.wide { (bar(have, languages)) }
                    }
                }
            }
        } }

        h2 { "Most languages know one thing about themselves" }
        div.scroll { table.facets {
            tbody {
                @for (score, count) in spread.into_iter().enumerate() {
                    @if count > 0 {
                        tr {
                            td.num { (score) " of 8" }
                            td.num { (count) }
                            td.wide { (bar(count, widest)) }
                        }
                    }
                }
            }
        } }
        h2 { "The part that does something" }
        p {
            "Where several languages claim an extension, langbank carries rules that
             read the bytes instead. " (link("/identify", "Paste a file"))
            " — it says which rule fired, not just which language."
        }
    };
    page("Languages, ecosystems and toolchains as data", &[], body)
}
