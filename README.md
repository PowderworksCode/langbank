# Langbank

Language, ecosystem and toolchain data: what a language is, how to recognise
it, what conventions it carries, which ecosystem publishes it, and what its
tooling can be asked.

Everything here is a registry over static data plus the few functions needed to
look something up. Nothing here walks a filesystem, spawns a process, or parses
a source file.

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
| languages | 29 |
| ecosystems | 5 — cargo, npm, pnpm, yarn, bun |
| tool profiles | 17, with 31 command patterns — what an invocation does and what it produces |
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

## The data is data

Languages live in `data/` as TOML and `build.rs` generates the same `&'static`
tables they used to be written as by hand. Nothing downstream pays for the
move: the statics are identical, the registration is identical, and there is no
runtime parsing.

```
data/
  comment-syntax.toml       tables shared by languages that comment alike
  facets.toml               reusable source surfaces
  artifacts.toml            what a build produces
  languages/rust.toml       one file per language
  ecosystems/cargo.toml     one file per ecosystem, with the directories it generates
  tools/cargo.toml          one file per tool, with its command patterns
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
