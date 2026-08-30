---
title: Reference
description: Every registry langbank carries, rendered from the data itself.
order: 5
---

Dry, complete, and current by construction: the build renders every table
below from [the manifest](/langbank.json) the crate exports, and CI fails
when that manifest no longer matches the binary. Come here to look something
up — the [tutorial](/getting-started/) and [guides](/guides/) are for
learning.

- [Crates](/reference/crates/) — The workspace members and what each may
  depend on.
- [Facets](/reference/facets/) — The eight things langbank can know about a
  language.
- [Languages](/reference/languages/) — Every language, its role, and what it
  knows.
- [Ecosystems](/reference/ecosystems/) — Manifests, lockfiles, and the
  registries they resolve against.
- [Toolchains](/reference/toolchains/) — What builds, tests, formats and
  lints each language.
- [Package registries](/reference/package-registries/) — How identity is
  spelled, per purl type.
- [Tool profiles](/reference/tools/) — Programs a repository invokes and the
  files that configure them.
- [Gaps](/reference/gaps/) — The questions langbank declined to answer, and
  the reason it declined each one.
