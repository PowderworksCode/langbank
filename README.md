# Langbank

Language, ecosystem and toolchain data: what a language is, how to recognise
it, what conventions it carries, which ecosystem publishes it, and what its
tooling can be asked.

Everything here is a registry over static data plus the few functions needed to
look something up. Nothing here walks a filesystem, spawns a process, or parses
a source file.

## Attribution

Langbank's language data is its own, and some of it began as facts checked
against permissively licensed upstream projects. With thanks:

| upstream | licence | what it contributed |
|---|---|---|
| [github-linguist/linguist](https://github.com/github-linguist/linguist) | MIT | language names, extensions, filenames, interpreters |
| [package-url/purl-spec](https://github.com/package-url/purl-spec) | MIT | package registry types, canonical hosts, identity rules |
| [XAMPPRocky/tokei](https://github.com/XAMPPRocky/tokei) | MIT / Apache-2.0 | comment syntax, extensions |
| [boyter/scc](https://github.com/boyter/scc) | MIT | comment syntax, extensions |
| [neovim/nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) | Apache-2.0 | language servers, commands, root markers |
| [mason-org/mason-registry](https://github.com/mason-org/mason-registry) | Apache-2.0 | tool roles and distribution |
| [analysis-tools-dev/static-analysis](https://github.com/analysis-tools-dev/static-analysis) | MIT | linters and formatters per language |
| [dependabot/dependabot-core](https://github.com/dependabot/dependabot-core) | MIT | package ecosystems, manifests, lockfiles |

Langbank deliberately carries **no data from copyleft-licensed projects**. That
is a standing decision rather than an oversight — see `docs/sources.md`, which
records which upstreams were considered and why each was taken or left.

## Why it is its own repository

It sits at the bottom of the fleet. Entl names languages while walking a tree,
treebank names them when it publishes a grammar, and propbank names them when
it observes a program. All three need the same vocabulary, and none of them
should have to depend on another to get it.

```
                    langbank
                 ↑      ↑      ↑
              entl  treebank  propbank
```

Every arrow points in. Nothing here depends on anything else in the fleet.

## On the name

`treebank` · `propbank` · `langbank`. The first two are annotated corpora in the
computational-linguistics sense — treebank derives its facts by sweeping a
corpus of source, propbank derives its by running compilers over programs. This
one is not derived from anything. It is hand-curated, and it is relational:
ecosystems point at languages, tools point at languages and artifacts,
languages point at facets and at the languages they supersede.

By the field's own convention that makes it a **net**, not a bank — a WordNet
rather than a PropBank. `-net` lost anyway, because in 2026 it reads as *neural
network* to every engineer who will ever type it, and a name that mispatterns on
sight costs more than a taxonomic inaccuracy a footnote can fix. Consider this
the footnote.

`lang-` is likewise approximate and less so than it looks: languages are the
spine here and everything else hangs off them. npm is JavaScript's ecosystem,
rustc is Rust's toolchain, a `.napi` artifact is a Node thing. Every registry
below is reachable from a language.

**Nothing here is derived from a corpus.** If you came looking for the pipeline
that regenerates it, there isn't one, and that is the point — this is the stable
leaf the rest of the fleet names things in.

## What is in it today

Lifted from `entl-codebase/src/profiles`, essentially unchanged:

| registry | count |
|---|---|
| languages | 827 |
| package registries | 42, aligned with purl |
| ecosystems | 19 package managers across 13 languages |
| tool profiles | 17, with 31 command patterns — what an invocation does and what it produces |
| toolchains | 1,118 — compilers and runtimes with version probes, language servers with root markers, and linters, formatters and debuggers across 117 languages |
| artifacts | binary, napi, site, tauri |
| facets | structured-code, style-host, component-host |
| conventions | test layout, inline-test detection, typecheck defaults |
| verbosity | measured relative verbosity per language and per language pair |
| traversal | registered pruning directories |

```rust
use langbank::{detect_language, language_profile, verbosity_ratio};

detect_language(Path::new("src/main.rs"), None);          // -> rust, by extension
detect_language(Path::new("deploy"), Some(b"#!/bin/sh")); // -> shell, by shebang
language_profile("rust").and_then(|p| p.conventions);     // test layout, inline tests
verbosity_ratio("rust", "typescript");                    // measured, corpus-versioned
```

Registration goes through `inventory`, so a downstream crate can add profiles
without editing this one.

## Every language, one shape

Langbank carries **827 languages**, one file each. They differ in depth, not in
kind: a thin entry is a name and a way to recognise it, a modelled one adds
conventions, facets and comment syntax, and enriching a language means editing
its file rather than promoting it between tiers.

```toml
# data/languages/cobol.toml — thin, for now
id = "cobol"
display-name = "COBOL"
role = "programming"
extensions = ["cbl", "ccp", "cob", "cpy"]
```

There is no curated-versus-imported flag, and no per-language attribution. How
well a language is modelled is read off its data — does it have conventions? —
because every language is meant to be fully modelled eventually, and a tag
recording where a fact came from would outlive its usefulness. Credit belongs in
one place at the top of this file, not scattered across 827 of them.

### Contested tokens

Completeness brings collisions: **176 of 1,478 extensions are claimed by more
than one language**. `.inc` belongs to twelve, `.h` to three, `.rs` to Rust,
RenderScript and XML.

A contest is settled only when exactly one claimant declares the token
`primary-extensions`. Otherwise detection **returns nothing** — guessing without
reading the file is a wrong answer where declining is merely an unhelpful one —
and `languages_claiming_extension` hands a consumer the candidates so it can
decide for itself. 28 contests are settled; 148 are left honestly open.

### Toolchains are facts about programs, and they were measured

A toolchain entry says which program implements a language, how to find out
whether it is installed and at what version, and how to ask it for
machine-readable diagnostics. Langbank never runs any of it — it supplies the
arguments, the stream and the pattern, and the consumer executes.

```toml
# data/toolchains/java.toml
[version]
arguments = ["-version"]
# stderr, where javac with the same flag writes to stdout
stream = "stderr"
pattern = 'version "(\d+(?:\.\d+)*)'
```

Three of these were measured rather than assumed, and none would have been
guessed right:

- **`java -version` writes to stderr; `javac -version` writes to stdout.** Same
  vendor, same flag.
- **`clang` is frequently absent where clang is installed** — packaged builds
  land as `clang-21`. `programs` is a fallback chain, and on the machine this
  was written the entry verified via `clang-21` with no `clang` present at all.
- **GCC prints its version twice**, once inside the distribution's package
  string and once at the end. The pattern is anchored to the end because those
  two agree only by convention.

`tools/verify-toolchains.py` runs every probe against whatever is installed and
reports; it skips absent programs rather than failing, because no machine has
all of them. 14 of 16 verified where this was written.

**Root markers belong to the program, not to the language.** clangd decides a
project by `compile_commands.json`, deno by `deno.json`, pyright by
`pyrightconfig.json` — three conventions, one of which is not even about the
same language. Unioning them per language was tried and produces noise: most
servers listing `rust` among their filetypes are generic formatters and
spellcheckers, and in that pile `Cargo.toml` is outvoted by `dprint.json`.

### Where a tool comes from

Mason is the inverse index of lspconfig: lspconfig knows how to *run* a tool,
mason knows what it *is* and how it is *published* — in purl, which is the
vocabulary `data/registries/` already carries.

```toml
categories = ["linter", "formatter", "language-server"]

[distribution]
registry = "github"
package = "astral-sh/ruff"
```

A tool is frequently several things at once, so `categories` is a list and
`kind` is only the primary role. `distribution.registry` resolves to a purl type
where purl defines one — mason publishes some packages under `openvsx`, which it
does not, so those resolve to nothing rather than to something wrong.

### Staying current

Upstream sources are what langbank is *checked against*, never what it defers
to:

```sh
tools/sync-linguist.py check     # CI: fails if linguist knows something we do not
tools/sync-linguist.py create    # writes a file for each language we lack
```

`check` compares every language and every extension, filename and interpreter.
`create` only ever writes files that do not exist — a hand-written entry with a
missing token is reported for a person to add, never silently rewritten. The
same shape of tool is how any further source gets absorbed.

## The data is data

Languages live in `data/` as TOML and `build.rs` generates the same `&'static`
tables they used to be written as by hand. Nothing downstream pays for the
move: the statics are identical, the registration is identical, and there is no
runtime parsing.

```
data/
  comment-syntax.toml       tables shared by languages that comment alike (76)
  facets.toml               reusable source surfaces
  artifacts.toml            what a build produces
  languages/rust.toml       one file per language, all 827 of them
  sources/linguist.toml     upstreams checked against, pinned by revision and digest
  ecosystems/cargo.toml     one file per package manager, with what it generates
  registries/npm.toml       one file per purl type: where package identities live
  tools/cargo.toml          one file per tool, with its command patterns
  toolchains/rustc.toml     one file per program: version probe, diagnostics
```

```toml
id = "typescript"
extensions = ["ts", "tsx", "mts", "cts"]
source-extensions = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"]
facets = ["structured-code", "style-host", "component-host"]
comments = "javascript"        # names a shared table
supersedes = ["javascript"]    # names another language

[[conventions.inline-test]]
starts-with = ["import "]
contains-any = ['from "vitest"', "from 'vitest'", "@jest/globals", "node:test"]
indicator = "test framework import"
```

Inline-test detection was the one thing that made these profiles not-data: a
function pointer per language. Both detectors that existed turned out to be the
same shape — a line prefix, sometimes narrowed by something the same line must
also contain — so the shape became a rule and the languages keep only their
tables. The interpreter evaluates line-major then rule-major, first match wins,
which is the order the hand-written detectors used, so the answers do not
change.

## Direction

Stated as intent rather than schedule.

1. **Bootstrap detection breadth from GitHub linguist**, whose `languages.yml`
   is MIT and already the de-facto standard, keeping the hand-modelled depth
   layer for the languages this fleet actually works on. Two layers, two
   evidence bars, and the difference recorded rather than blurred.
2. **Model toolchains**, which exist nowhere today: compiler identity and
   version probes, invocation patterns for build/test/typecheck/format/lint,
   registry and popularity sources, and machine-readable diagnostic formats.
   treebank has all of this hardcoded per language in Rust, and propbank needs
   the version probes for its staleness checks.
3. **Absorb treebank's registry data** — crates.io dumps, npm, Maven Central,
   NuGet, Debian popcon, `packages.ecosyste.ms` — which is `rank`/`resolve`
   today and is plainly data.

## Provenance

`profiles/` came from entl, whose design doc drew the line this repository
makes structural:

> "Language profiles and ecosystem profiles are separate registries. An
> ecosystem role is not a language, and a language is not an ecosystem."

One behavioural change during the lift: `DependencyPinPolicy::classify` took a
parsed `Dependency` and now takes a `DependencySource` and a requirement.
A pin policy needs the taxonomy and the spec; the parsed record belongs to
whoever read the manifest, and borrowing it would drag manifest parsing down
here. Covered by `tests/registries.rs`.
