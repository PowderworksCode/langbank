---
title: Keep the data honest
description: What gets checked against upstream, what gets recorded as absent, and why.
order: 3
---

A hand-curated registry earns trust by showing its work. Langbank does it
three ways: the data compiles, upstreams are checked against, and absences
are recorded.

## The data is the build

`data/**/*.toml` is the source of truth. `build.rs` compiles it into
`&'static` tables, so a malformed entry is a build failure rather than a
runtime surprise — and an empty registry cannot compile at all, because a
consumer that sees a language-free world with no error is the worst outcome
on the menu.

## Upstreams are checked against, not deferred to

Langbank owns every fact it carries, and some of those facts began as claims
checked against permissively licensed upstream projects —
[credited here](/project/sources/). The `langbank-sync` crate re-runs those
checks: `check` fails when an upstream knows something langbank does not, and
`create` drafts the data change that would close the difference. CI runs the
checks on a schedule, so drift is a report rather than a discovery.

## Absences are recorded

When sources disagree, when only one source makes a claim and nothing
corroborates it, or when a fact is excluded on purpose, langbank records
[a gap](/reference/gaps/) instead of guessing. A registry that silently omits
what it does not know cannot be told apart from one nobody has filled in.

## What the website adds

The site is part of the same discipline. The crate exports its registries as
[one JSON manifest](/langbank.json), the committed copy is diffed against the
binary in CI, and every [reference page](/reference/) is rendered from it at
build time. A data change that forgets to regenerate the manifest fails the
build; a reference page cannot go stale because it is never written by hand.
