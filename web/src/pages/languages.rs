//! The language index and one page per language.

use crate::render::{code, codes, link, page, row};
use langbank::{Knowledge, LanguageProfile};
use maud::{Markup, html};

/// Eight cells, one per facet, filled where langbank carries it.
///
/// The index used to show a name and an extension, which told a reader nothing
/// about whether the entry was worth opening. This is the difference between a
/// language langbank has a paragraph about and one it has a filename for, and
/// it is legible at a glance down a column.
fn marks(knowledge: &Knowledge) -> Markup {
    html! {
        span.marks title=(summary(knowledge)) {
            @for (facet, have) in knowledge.facets() {
                @if have { i.on data-facet=(facet.name()) {} } @else { i {} }
            }
        }
    }
}

fn summary(knowledge: &Knowledge) -> String {
    let carried: Vec<&str> = knowledge
        .facets()
        .filter(|(_, have)| *have)
        .map(|(facet, _)| facet.name())
        .collect();
    if carried.is_empty() {
        return "nothing but a name".into();
    }
    format!("{} of 8: {}", carried.len(), carried.join(", "))
}

pub fn index() -> String {
    let mut all: Vec<&LanguageProfile> = langbank::language_profiles().to_vec();
    all.sort_by_key(|l| l.display_name.to_lowercase());

    let body = html! {
        h1 { (all.len()) " languages" }
        p.lede {
            "A language is here because a source named it, not because it seemed
             important. The cells are the eight things langbank can know about one —
             hover for which."
        }
        ul.grid.marked {
            @for language in &all {
                @let knowledge = Knowledge::of(language);
                li {
                    (marks(&knowledge))
                    (link(&format!("/languages/{}", language.id), language.display_name))
                    @if let Some(hint) = language.primary_extensions.first()
                        .or_else(|| language.extensions.first())
                    {
                        " " small { "." (hint) }
                    }
                }
            }
        }
    };
    page("Languages", &[("/", "langbank")], body)
}

/// A list of `.ext` spans.
fn dotted(values: &[&str]) -> Markup {
    codes(&values.iter().map(|e| format!(".{e}")).collect::<Vec<_>>())
}

/// `None` where langbank knows nothing, so `row` can omit it rather than render
/// an empty cell.
fn some(condition: bool, markup: Markup) -> Option<Markup> {
    condition.then_some(markup)
}

pub fn detail(id: &str) -> Option<String> {
    let language = langbank::language_profile(id)?;
    let toolchains = langbank::toolchains_for(language);
    let ecosystems: Vec<&'static langbank::EcosystemProfile> = langbank::ecosystem_profiles()
        .iter()
        .filter(|e| e.implied_languages.iter().any(|l| l.id == language.id))
        .copied()
        .collect();
    // Contested extensions are the interesting part: they are where a name is
    // not an answer and the content rules take over.
    let contested: Vec<(&str, Vec<&str>)> = language
        .extensions
        .iter()
        .filter(|e| langbank::languages_claiming_extension(e).len() > 1)
        .map(|e| {
            let others = langbank::languages_claiming_extension(e)
                .iter()
                .filter(|l| l.id != language.id)
                .map(|l| l.display_name)
                .collect();
            (*e, others)
        })
        .collect();
    let gaps = langbank::gaps_for(language.id);

    let knowledge = Knowledge::of(language);
    let body = html! {
        h1 { (language.display_name) }
        ul.knows {
            @for (facet, have) in knowledge.facets() {
                li.have[have] {
                    (facet.name()) " " span.dim { (facet.purpose()) }
                }
            }
        }
        dl {
            (row("id", Some(code(language.id))))
            (row("role", Some(html! { (format!("{:?}", language.role).to_lowercase()) })))
            (row("claims first", some(!language.primary_extensions.is_empty(),
                dotted(language.primary_extensions))))
            (row("extensions", some(!language.extensions.is_empty(),
                dotted(language.extensions))))
            (row("filenames", some(!language.filenames.is_empty(), codes(language.filenames))))
            (row("shebangs", some(!language.shebangs.is_empty(), codes(language.shebangs))))
            (row("config files", some(!language.config_files.is_empty(),
                codes(language.config_files))))
            (row("facets", some(!language.facets.is_empty(), html! {
                @for facet in language.facets {
                    span.tag title=(facet.description) { (facet.id) }
                }
            })))
            (row("supersedes", some(!language.supersedes.is_empty(), html! {
                @for (index, other) in language.supersedes.iter().enumerate() {
                    @if index > 0 { ", " }
                    (link(&format!("/languages/{}", other.id), other.display_name))
                }
            })))

            @if let Some(comments) = language.comments {
                (row("line comment", some(!comments.line.is_empty(), codes(comments.line))))
                (row("block comment", some(!comments.block.is_empty(), html! {
                    @for (index, (open, close)) in comments.block.iter().enumerate() {
                        @if index > 0 { ", " }
                        (code(open)) " … " (code(close))
                    }
                })))
                (row("doc comment", some(!comments.documentation.is_empty(),
                    codes(comments.documentation))))
                (row("string quotes", some(!comments.quotes.is_empty(),
                    codes(&comments.quotes.iter().map(char::to_string).collect::<Vec<_>>()))))
                (row("multiline quotes", some(!comments.multi_quotes.is_empty(),
                    codes(comments.multi_quotes))))
            }

            (row("toolchains", some(!toolchains.is_empty(), html! {
                @for (index, toolchain) in toolchains.iter().enumerate() {
                    @if index > 0 { ", " }
                    (link(&format!("/toolchains#{}", toolchain.id), toolchain.display_name))
                }
            })))
            (row("ecosystems", some(!ecosystems.is_empty(), html! {
                @for (index, ecosystem) in ecosystems.iter().enumerate() {
                    @if index > 0 { ", " }
                    (link(&format!("/ecosystems#{}", ecosystem.id), ecosystem.display_name))
                }
            })))
            (row("shares", some(!contested.is_empty(), html! {
                @for (index, (extension, others)) in contested.iter().enumerate() {
                    @if index > 0 { br; }
                    (code(&format!(".{extension}"))) " with " (others.join(", "))
                }
            })))
        }

        @if !gaps.is_empty() {
            h2 { "What is not known, and why" }
            div.scroll { table {
                thead { tr { th { "facet" } th { "reason" } th { "note" } } }
                tbody {
                    @for gap in &gaps {
                        tr {
                            td { (code(gap.facet)) }
                            td { (format!("{:?}", gap.reason)) }
                            td { (gap.note) }
                        }
                    }
                }
            } }
        }
    };
    Some(page(
        language.display_name,
        &[("/", "langbank"), ("/languages", "languages")],
        body,
    ))
}
