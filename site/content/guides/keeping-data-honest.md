---
title: Keep the data honest
description: What the sync tools check against upstream, what langbank records as absent, and why.
order: 3
---

A hand-curated registry earns trust by showing its work. Langbank shows it
three ways: the data has to compile, the sync tools check it against
upstreams, and the gaps record what it declines to answer.

## The data is the build

`data/**/*.toml` is the source of truth. `build.rs` compiles it into
`&'static` tables, so a malformed entry is a build failure rather than a
runtime surprise — and an empty registry cannot compile at all, because a
consumer that sees a language-free world with no error is the worst outcome
on the menu.

## Checking against upstreams, deferring to none

Langbank owns every fact it carries, and some of those facts began as claims
verified against permissively licensed upstream projects —
[credited here](/project/sources/). The `langbank-sync` crate re-runs the
verification: `check` fails when an upstream knows something langbank does
not, and `create` drafts the data change that would close the difference. CI
runs the checks on a schedule, so drift is a report rather than a discovery.

## Recording absences

When sources disagree, when only one source makes a claim and nothing
corroborates it, or when langbank excludes a fact on purpose, it records
[a gap](/reference/gaps/) instead of guessing. A registry that silently omits
what it does not know looks identical to one nobody has filled in.

## What the website adds

The site is part of the same discipline. The crate exports its registries as
[one JSON manifest](/langbank.json), CI diffs the committed copy against the
binary, and the build renders every [reference page](/reference/) from it. A
data change that forgets to regenerate the manifest fails the build; a
reference page cannot go stale because nothing writes it by hand.
