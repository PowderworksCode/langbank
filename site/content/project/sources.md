---
title: Sources
description: Which upstreams contributed facts, which langbank refused, and the licence stance that decides.
order: 1
---

Langbank's language data is its own, and some of it began as facts checked
against permissively licensed upstream projects. This page is the public
account; the working ledger — sync tooling, ordering, reconciliation rules —
lives in the repository at
[docs/sources.md](https://github.com/PowderworksCode/langbank/blob/main/docs/sources.md).

## The standing decision

**Langbank carries no data from copyleft-licensed projects.** Not as a copy,
not as a CI checker, not at all for now. That rules out four of the richest
datasets, and it rules them out knowingly:

| project | licence | why it hurts |
|---|---|---|
| helix-editor/helix | MPL-2.0 | the single richest dataset after linguist — file types, shebangs, root markers, comment tokens, grammars, language servers, formatters, debuggers, tied together per language |
| AlDanial/cloc | GPL-2.0 | decades of extension and comment-filter definitions, and an independent lineage |
| renovatebot/renovate | AGPL-3.0 | cross-ecosystem dependency-manager knowledge |
| librariesio/libraries.io | AGPL-3.0 | package-manager adapters across ecosystems |

Langbank is the MIT leaf that every other repository in the fleet links. A
checker that only compares and never copies would probably be fine — facts are
not what copyright reaches; a curated selection and arrangement is — but
"probably fine" is not a property worth having in the crate everything depends
on. The decision gets revisited deliberately or not at all.

## Sources taken

Verified licences, not assumed ones:

| upstream | licence | what it contributed |
|---|---|---|
| [github-linguist/linguist](https://github.com/github-linguist/linguist) | MIT | language names, extensions, filenames, interpreters, content rules |
| [package-url/purl-spec](https://github.com/package-url/purl-spec) | MIT | package registry types, canonical hosts, identity rules |
| [XAMPPRocky/tokei](https://github.com/XAMPPRocky/tokei) | MIT / Apache-2.0 | comment syntax, extensions |
| [boyter/scc](https://github.com/boyter/scc) | MIT | comment syntax, extensions |
| [neovim/nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) | Apache-2.0 | language servers, commands, root markers |
| [mason-org/mason-registry](https://github.com/mason-org/mason-registry) | Apache-2.0 | tool roles and distribution |
| [analysis-tools-dev/static-analysis](https://github.com/analysis-tools-dev/static-analysis) | MIT | linters and formatters per language |
| [dependabot/dependabot-core](https://github.com/dependabot/dependabot-core) | MIT | package ecosystems, manifests, lockfiles |

Langbank checks against upstreams rather than deferring to them: it owns
every fact it carries, `langbank-sync` re-runs the checks on a schedule, and a
difference is a report to act on rather than an automatic import.

## Agreement counts only when the sources are independent

go-enry stays out because it ports linguist and so carries no independent
facts. tokei and scc looked like they might share ancestry and turned out not
to: on the languages both carry — 187 of them — they agree on 77% of
extension sets, 93% of line comments and 89% of block comments. Two corpora agreeing on
99.9% of anything would be one corpus wearing two hats; these are far enough
apart to be two, so their agreement is evidence — and the slice they disagree
about is where the decisions live.

When sources disagree, langbank records [a gap](/reference/gaps/) rather than
averaging the difference away.
