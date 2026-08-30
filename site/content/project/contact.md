---
title: Contact
description: "How to reach the langbank maintainers: corrections, additions, and security issues."
order: 3
---

Langbank has no support inbox; GitHub is the channel. Everything happens in
public except security reports, which have a private route.

## Corrections and additions

A wrong extension, a missing toolchain, a comment syntax langbank lacks —
open an issue on the repository, or a pull request against `data/`: the TOML
there is the source of truth, and `build.rs` turns it into the tables every
consumer reads.
[GitHub Issues](https://github.com/PowderworksCode/langbank/issues).

A claim needs a source. Facts here are checked against upstreams and
disagreements are [recorded rather than guessed at](/reference/gaps/), so
"my compiler says so" with a link beats "everyone knows".

## Security problems

Anything exploitable should be reported privately via
[GitHub Security Advisories](https://github.com/PowderworksCode/langbank/security/advisories/new)
rather than a public issue.
