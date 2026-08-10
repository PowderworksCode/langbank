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
| package-url/purl-spec | MIT | ecosystem naming | next |
| XAMPPRocky/tokei | MIT / Apache-2.0 | language identity, comment syntax | queued |
| boyter/scc | MIT | language identity, comment syntax | queued |
| neovim/nvim-lspconfig | Apache-2.0 | language → server, root markers | after toolchains |
| mason-org/mason-registry | Apache-2.0 | tool → languages, categories | after toolchains |
| dependabot/dependabot-core | MIT | ecosystem manifests, registries | after toolchains |
| analysis-tools-dev/static-analysis | MIT | language → analyzers | later |
| git-pkgs/brief | MIT | language → toolchain, 22 categories | schema reference |
| codemirror/language-data | MIT, archived | aliases | low value |

**go-enry is skipped.** It is a port of linguist, so it carries no independent
facts; its interest is as precedent for a Rust core with bindings, not as data.

**brief is the closest analogue to langbank** and is treated as a schema
reference rather than a feed. Absorbing it wholesale would mostly be re-hosting
someone else's work, and its premise differs: brief inspects a project to answer
"what is configured here", where langbank supplies the vocabulary that question
is asked in.

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
   `check` / `create` split linguist uses. `check` reports what upstream knows
   and langbank does not and runs in CI; `create` only ever writes files that do
   not exist, because a hand-written entry carries conventions no importer should
   touch.
3. **Contests are resolved by `primary-extensions`, and only by that.** It needs
   no redesign for more sources — corroboration is what justifies each new claim.
4. **Disagreement between sources is recorded, never silently resolved.** Two
   sources differing on a language's role is a finding for a person, in the same
   spirit as the 148 contests langbank already declines to guess at.

### Agreement is only evidence if the sources are independent

go-enry derives from linguist. scc, tokei and cloc may share ancestry with each
other. Before treating "three sources agree" as confidence, establish that they
are three sources: if two agree on 99.9% of extensions they are one corpus
wearing two hats, and counting it twice is how a registry becomes confidently
wrong. The first corroboration should measure this rather than assume it.

## Order

```
0. purl-spec ids           before anything else accretes: renaming ecosystems
                           gets more expensive with every source added
1. tokei + scc             comment syntax is langbank's thinnest area — 11 tables
                           for 827 languages — and this is the first corroboration
2. independence audit      is agreement actually evidence?
3. TOOLCHAIN MODEL         the hinge; four sources below wait on it, and propbank
                           needs the version probes regardless
4. nvim-lspconfig          root markers, which langbank has no concept of
5. mason-registry          tool → language, categories, distribution
6. dependabot-core         ecosystem manifests, lockfiles, registries
7. static-analysis         analyzer coverage per language
```
