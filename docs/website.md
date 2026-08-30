# The website, and the server it replaced

langbank.dev is `site/`: a tree of markdown that
[`@powderworks/docs`](https://github.com/PowderworksCode/docs) builds into
plain files. A Cloudflare Worker serves them and does one thing beyond that —
it hands an agent the markdown twin of a page when the request asks for
`text/markdown`.

The build renders every page that lists data from `site/content/langbank.json`,
which `scripts/data-manifest.sh` exports from the compiled tables. `docs.yml`
diffs the committed manifest against the binary, so a data change that forgets
to regenerate it fails CI rather than reaching a reader. Nobody writes anything
on this site twice.

## What stood here before

An axum server, `langbank-web`, rendering the same registries from the same
`&'static` tables, deployed to Fly as the `langbank` app with a preview app per
pull request. The commit that added this file removed it.

It went for the reason somebody wrote it: the registries are static. Every page
it served came out of tables fixed at compile time, so a machine had to stay
up, a Dockerfile had to build, and two workflows had to hold a Fly token — all
to compute on demand a set of pages that never change between commits.
Rendering them at build time costs a build step, and drops the account, the
runtime and the deploy secret.

Git keeps it. To read it:

    git show 5a3ca3e:web/src/main.rs
    git log --oneline -- web/

## What the site does not carry

Three things the server had have no counterpart here, and deserve naming rather
than discovery:

| gone | what it did | where it stands |
|---|---|---|
| `/languages/<id>` and the other per-entity pages | one page per language, ecosystem, toolchain, registry and tool, with its cross-references | the reference tables carry every entity and its facts; the per-entity page does not exist |
| `/identify` | paste a file, run the content rules against it, see which rule fired | `langbank-detect` runs them; nothing hosted does |
| `/health` | registry sizes, for Fly's health check | there is no machine to check |

The tables are the substance and they survived. The interactive rule runner is
the real loss, and the only thing here a reader cannot get from the data. It
wants a form and something to run `langbank-detect` behind it, which is a
Worker rather than a server, and nobody has written one.
