---
title: Look things up
description: The registries beyond languages, and the lookups that connect them.
order: 2
---

Languages are the spine and everything else hangs off them: npm is
JavaScript's ecosystem, rustc is Rust's toolchain, a `.napi` artifact is a
Node thing. Every registry is reachable from a language.

## From a language outward

```rust
use langbank::{language_profile, toolchains_for};

let rust = language_profile("rust").unwrap();
let tools = toolchains_for(rust);           // compilers, linters, formatters …
let comments = langbank::comment_syntax("rust"); // `//` and `/* */`
```

A profile also answers structural questions directly: the extensions and
filenames it claims, its conventions for test layout, and the languages it
supersedes — TypeScript supersedes JavaScript, which is a different claim from
linguist's statistical `groups_under`.

## Ecosystems

An [ecosystem](/reference/ecosystems/) is a manifest, the lockfiles that pin
it, and the registry it resolves against — what tells a walker that a
directory is a project rather than a folder of files. Where a manifest alone
cannot settle ownership — four package managers read the same `package.json` —
selector files carry the decision.

## Toolchains and tools

A [toolchain](/reference/toolchains/) names the programs that invoke it, in
preference order, and the command that asks its version. The probe is data:
langbank states it, the caller runs it. A
[tool profile](/reference/tools/) goes the other way — from a program seen in
a CI log or a lockfile to what it does and the files that configure it.

## Package registries

A [package registry](/reference/package-registries/) says how identity is
spelled: whether a namespace is required, what case-folds, and where a name
resolves by default — aligned with [purl](https://github.com/package-url/purl-spec)
types, so `pkg:cargo/serde` means here what it means everywhere else.

## The data without the crate

Everything the reference pages show is rendered from
[one JSON manifest](/langbank.json) the crate exports. A consumer that wants
the tables without a Rust dependency can fetch that file; its `schema` key
names the shape it promises.
