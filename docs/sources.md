# Upstream sources

Which projects langbank takes facts from, which it refuses to, and in what
order the rest arrive. Written down because "why isn't helix in here, it has
exactly what we need" is a question that will be asked more than once.

## The standing decision

**Langbank carries no data from copyleft-licensed projects.** Not as a copy,
not as a CI checker, not at all for now.

That rules out four of the richest datasets, and it rules them out knowingly:

| project | licence | why it hurts |
|---|---|---|
| helix-editor/helix | MPL-2.0 | `languages.toml` is the single richest dataset after linguist — file types, shebangs, root markers, comment tokens, grammars, language servers, formatters, debuggers, all tied together per language |
| AlDanial/cloc | GPL-2.0 | decades of extension and comment-filter definitions, and an independent lineage |
| renovatebot/renovate | AGPL-3.0 | cross-ecosystem dependency-manager knowledge |
| librariesio/libraries.io | AGPL-3.0 | package-manager adapters across ecosystems |

MPL-2.0 is file-level copyleft and GPL/AGPL are stronger still, and langbank is
the MIT leaf that every other repository in the fleet links. A checker that only
compares and never copies would probably be fine — facts are not what copyright
reaches, a curated selection and arrangement is — but "probably fine" is not a
property worth having in the crate everything depends on. Revisit deliberately
or not at all.

## Sources taken, and available

Verified licences, not assumed ones. tokei is dual-licensed and purl-spec is
MIT; GitHub reports both as `NOASSERTION` and is wrong to.

| project | licence | shape | status |
|---|---|---|---|
| github-linguist/linguist | MIT | language identity | **absorbed** |
| package-url/purl-spec | MIT | package registries | **absorbed** |
| XAMPPRocky/tokei | MIT / Apache-2.0 | comment syntax, extensions | **absorbed** |
| boyter/scc | MIT | comment syntax, extensions | **absorbed** |
| neovim/nvim-lspconfig | Apache-2.0 | language servers, root markers | **absorbed** |
| mason-org/mason-registry | Apache-2.0 | tool roles, distribution | **absorbed** |
| dependabot/dependabot-core | MIT | package ecosystems, manifests, lockfiles | **absorbed** |
| analysis-tools-dev/static-analysis | MIT | linters and formatters per language | **absorbed** |
| git-pkgs/brief | MIT | language → toolchain, 22 categories | schema reference |
| codemirror/language-data | MIT, archived | aliases | low value |

**go-enry is skipped.** It is a port of linguist, so it carries no independent
facts; its interest is as precedent for a Rust core with bindings, not as data.

**brief is the closest analogue to langbank** and is treated as a schema
reference rather than a feed. Absorbing it wholesale would mostly be re-hosting
someone else's work, and its premise differs: brief inspects a project to answer
"what is configured here", where langbank supplies the vocabulary that question
is asked in.

## What purl actually settled

Aligning with purl was expected to be a rename and turned out to be a model
split. A purl type names the **registry** a package identity lives in —
`pkg:npm/lodash@4` — and langbank's "ecosystem" was conflating that with the
**manager** that reads the manifest. Only two of five langbank ecosystems were
purl types, because npm, pnpm, yarn and bun are four managers over one registry
and differ in lockfile, not in what a package is called.

So `data/registries/` carries the 42 purl types with their canonical hosts and
identity rules, and an ecosystem points at one. Nothing was renamed.

## Three shapes, which is what decides the order

- **Language identity** — linguist, tokei, scc, codemirror. Same shape as
  `data/languages/`, directly mergeable and mutually checkable.
- **Ecosystem knowledge** — purl-spec, dependabot-core. Feeds `data/ecosystems/`.
- **Tool corpora, inverse-shaped** — mason, nvim-lspconfig, static-analysis,
  brief. These are `tool → languages`, the inverse of langbank's language-centric
  registries, and most of them want a toolchain model that does not exist yet.

## How facts from several sources reconcile

One source needed no machinery. Several do.

1. **Attribution lives in the README, not the data.** Per-language `sources`
   tags were tried and removed: every language is meant to be fully modelled
   eventually, at which point the tag says nothing, and credit reads better in
   one place than scattered across 827 files.
2. **Each source gets its own `tools/sync-<source>.py`**, with the same
   `check` / `create` split linguist uses. `create` only ever writes files that
   do not exist, because a hand-written entry carries conventions no importer
   should touch.

   `check` runs in **`drift.yml`, on a schedule and on `main` — not on pull
   requests.** Seven upstreams move without asking, and a package appearing in
   mason has nothing to do with the change under review. Gating every branch on
   the state of somebody else's repository produces a red build nobody can fix
   from inside their own work, which is the same failure as a check that can
   never go green: everyone learns to ignore it.
3. **Contests are resolved by `primary-extensions`, and only by that.** It needs
   no redesign for more sources — corroboration is what justifies each new claim.
4. **Disagreement between sources is recorded, never silently resolved.** Two
   sources differing on a language's role is a finding for a person, in the same
   spirit as the 148 contests langbank already declines to guess at.

### Agreement is only evidence if the sources are independent — measured

go-enry derives from linguist and is skipped for that reason. tokei and scc were
suspected of sharing ancestry and turned out not to. On the 187 languages both
carry:

| | agreement |
|---|---:|
| identical extension sets | 77% |
| identical line comments | 93% |
| identical block comments | 89% |

Two corpora agreeing on 99.9% of anything would be one corpus wearing two hats.
These are far enough apart to be two, so their agreement is evidence — and the
7–23% they disagree about is where the decisions live rather than noise to
average away.

**The rule that follows.** Both agree, absorb. Only one carries it, absorb as a
single source, which is what linguist already is. Both carry it and differ,
report it and change nothing: 21 languages sit in that bucket, Lua among them,
where scc records six block-comment forms for its `--[==[` long brackets and
tokei records one. Neither is wrong and the smaller answer would quietly lose
the difference.

## Why dependabot went last

The documented order had dependabot before static-analysis. Measuring the two
reversed it, which is the reason to measure.

static-analysis publishes 755 tools as YAML with a stable schema — name,
categories, tags, license — and 666 of them were tools langbank did not know.
It overlaps mason far less than expected: mason indexes what an editor can
install, this indexes what an analyser community has written.

dependabot has the breadth — thirty-odd ecosystems against langbank's six —
but its facts are inside Ruby:

```ruby
def self.required_files_in?(filenames)
  filenames.include?("Cargo.toml")
end
```

That is extractable by regex over method bodies, per ecosystem, across fetchers
that do not share a shape. It is worth doing for the ecosystems langbank does
not yet model, and it is the one source where the extraction is a project rather
than an afternoon.

## Order

```
0. purl-spec               done: 42 registries, and the registry/manager split
                           it forced
1. tokei + scc             done: comment syntax 27 -> 220 languages, and the
                           independence audit folded in
2. TOOLCHAIN MODEL         done: 16 programs with version probes and diagnostic
                           formats, measured against installed tools
3. nvim-lspconfig          done: 266 servers with their root markers
4. mason-registry          done: 157 tools gained a distribution, 326 are new
5. static-analysis         done: 510 linters and formatters across 117 languages
6. dependabot-core         done: 13 package ecosystems, read with Ripper rather
                           than regex — following constant references took the
                           yield from 16 ecosystems to 27
```
