//! Which languages langbank knows a lot about, and which it barely knows at all.
//!
//! The figures are `langbank::coverage` and `langbank::distribution`, the same
//! ones `langbank-sync coverage` prints, so the site and the tool cannot drift
//! apart about what "covered" means.

use crate::render::{bar, link, page};
use langbank::{Facet, Knowledge, LanguageProfile, coverage, distribution};
use maud::html;

/// The languages at one score, listed rather than counted.
///
/// A count says 535 languages know one thing; naming them is what makes it
/// possible to go and fix one. Capped, because 535 links is a wall.
fn at_score(score: usize, cap: usize) -> (Vec<&'static LanguageProfile>, usize) {
    let mut found: Vec<&'static LanguageProfile> = langbank::language_profiles()
        .iter()
        .filter(|profile| Knowledge::of(profile).count() == score)
        .copied()
        .collect();
    found.sort_by_key(|profile| profile.display_name.to_lowercase());
    let total = found.len();
    found.truncate(cap);
    (found, total)
}

pub fn render() -> String {
    let total = langbank::language_profiles().len();
    let carried = coverage();
    let spread = distribution();
    let widest = spread.iter().copied().max().unwrap_or(1);

    let body = html! {
        h1 { "Coverage" }
        p.lede {
            "Eight things langbank can know about a language, and how many languages
             it knows each of. Nothing here is inferred — a facet counts when the
             data carries it."
        }

        div.scroll { table.facets {
            thead { tr { th { "facet" } th { "lets a consumer" } th.num { "have" } th.num { "lack" } th {} } }
            tbody {
                @for (facet, have) in Facet::ALL.into_iter().zip(carried) {
                    tr {
                        td { code { (facet.name()) } }
                        td.dim { (facet.purpose()) }
                        td.num { (have) }
                        td.num.dim { (total - have) }
                        td.wide { (bar(have, total)) }
                    }
                }
            }
        } }

        h2 { "By role, so the denominator means something" }
        p.dim {
            "The totals above set the wrong target on their own. "
            code { "ecosystem" } " reads as 802 languages waiting to be filled in — but
             JSON has no package manager and CSV has no compiler, and 270 of the 827
             are data, markup, documentation, stylesheet or build formats. This says
             which languages the question was even asked of."
        }
        div.scroll { table.facets {
            thead {
                tr {
                    th { "role" }
                    th.num { "languages" }
                    @for facet in Facet::ALL { th.num { (facet.name()) } }
                }
            }
            tbody {
                @for (role, count, carried) in langbank::coverage_by_role() {
                    tr {
                        td { (format!("{role:?}").to_lowercase()) }
                        td.num { (count) }
                        @for have in carried {
                            td.num { @if have == 0 { span.none { "—" } } @else { (have) } }
                        }
                    }
                }
            }
        } }

        h2 { "How many languages know how much" }
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

        h2 { "The best covered" }
        p.dim { "Where langbank has something to say about nearly everything." }
        @for score in (5..=8).rev() {
            @let (found, count) = at_score(score, 60);
            @if count > 0 {
                h3 { (score) " of 8 " span.dim { "— " (count) " languages" } }
                ul.grid {
                    @for profile in &found {
                        li { (link(&format!("/languages/{}", profile.id), profile.display_name)) }
                    }
                }
            }
        }

        h2 { "Barely known" }
        p.dim {
            "Named by a source and almost nothing else. These are where the next
             absorption has the most to add — and where "
            (link("/gaps", "a recorded gap"))
            " is the honest answer until someone looks."
        }
        @for score in 0..=1 {
            @let (found, count) = at_score(score, 60);
            @if count > 0 {
                h3 {
                    (score) " of 8 " span.dim { "— " (count) " languages" }
                    @if count > found.len() { span.dim { ", first " (found.len()) } }
                }
                ul.grid {
                    @for profile in &found {
                        li { (link(&format!("/languages/{}", profile.id), profile.display_name)) }
                    }
                }
            }
        }
    };
    page("Coverage", &[("/", "langbank")], body)
}
