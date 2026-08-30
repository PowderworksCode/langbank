---
title: About
description: What langbank is, why it sits at the bottom of the fleet, and who builds it.
order: 2
---

Langbank is the vocabulary the rest of the fleet speaks: which languages
exist, what files they claim, what builds them, and how each registry spells
package identity — as data, in a crate that depends on nothing.

It sits at the bottom deliberately. Entl names languages while walking a tree,
treebank names them when it publishes a grammar, and propbank names them when
it observes a program. All three need the same vocabulary, and none of them
should have to depend on another to get it. Every arrow points in: nothing
here depends on anything else in the fleet.

The data is hand-curated and relational rather than derived. Treebank and
propbank earn their `-bank` by sweeping corpora; this one is a registry —
ecosystems point at languages, tools point at languages and artifacts,
languages point at facets and at the languages they supersede. Some of it
began as facts checked against permissively licensed upstream projects, each
credited on the [sources page](/project/sources/), and langbank deliberately
carries no data from copyleft-licensed projects.

[The Powderworks Agentic Coding Consortium](https://powderworks.dev) builds
langbank, and [Zack](https://github.com/zmaril) maintains it. The code is
MIT-licensed and lives on
[GitHub](https://github.com/PowderworksCode/langbank), where the full commit
history is public.

If a language you know is thin here — a wrong extension, a missing toolchain,
a comment syntax langbank lacks — the [contact page](/project/contact/) says
how to get it fixed.

## Newsletter

Releases, and notes on building software with agents.

<iframe class="embed" title="Subscribe to the Powderworks newsletter" src="https://newsletter.powderworks.dev/embed" scrolling="no"></iframe>
