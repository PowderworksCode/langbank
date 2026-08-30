# Deploying langbank.dev

The site is `web/` — an axum server that renders every page from the `&'static`
tables `build.rs` compiles out of `data/`. There is no database, no volume and
no runtime data directory, which is what makes the deployment boring: a machine
that boots is a machine that is fully working, two machines serve byte-identical
pages, and a preview is a complete copy of production with different data
compiled into it.

## What deploys where

| | app | config | trigger |
|---|---|---|---|
| production | `langbank` → langbank.dev | `fly.toml` | merge to `main` |
| preview | `langbank-pr-<n>` | `fly.preview.toml` | any PR from this repo |

Production keeps one machine warm (`min_machines_running = 1`) so the first
visitor of the hour does not wait for a cold start. Previews idle-stop and wake
on request — with no state, waking is indistinguishable from never having
stopped, so a preview costs nothing while nobody is reading it.

## One-time setup

Neither workflow can do these; they need an account. `scripts/fly-setup.sh` runs
the whole sequence; what follows is what each step is for.

1. **A preview-scoped Fly token that can create apps.** Both workflows read
   `secrets.FLY_PREVIEW_TOKEN`. A deploy token scoped to a single app will
   deploy production and fail every preview, because a preview creates a new app
   each time.

   ```sh
   fly tokens create org --name langbank-ci
   gh secret set FLY_PREVIEW_TOKEN --repo PowderworksCode/langbank
   ```

2. **Optionally set `vars.FLY_ORG`.** Without it, the deploy workflow takes the
   first org the token can see, which is right for a single-org account and
   wrong the moment there are two.

3. **The domain.** Fly issues the certificate; DNS has to point at it first.

   ```sh
   fly ips list --app langbank            # after the first deploy
   # A     @    <the v4 address>
   # AAAA  @    <the v6 address>
   fly certs create langbank.dev --app langbank
   fly certs create www.langbank.dev --app langbank
   fly certs check langbank.dev --app langbank
   ```

   Until DNS points at Fly, `deploy.yml`'s **Custom domain** step reports and
   passes — the app's own address is what gates the deploy, because that is
   what the deploy actually controls.

   A domain registered but not pointed is the trap here. It still resolves, to
   the registrar's parking page, so nothing looks obviously wrong: port 80
   answers and 443 does not. Check where it actually points before assuming
   propagation:

   ```sh
   getent hosts langbank.dev     # Fly is 66.241.x / 149.248.x
   fly ips list --app langbank   # what it should be
   ```

## Why the build looks like that

`Dockerfile` uses cargo-chef so the dependency compile is its own layer keyed on
the lockfile rather than the source. A push that only changes a page or a TOML
file reuses the cooked layer instead of recompiling axum, tokio and hyper.

`data/` is an input to `build.rs`, so a data-only change correctly invalidates
the application layer and nothing earlier — which is the common case, and the
one worth being fast.

The runtime image is `debian:bookworm-slim` plus the binary and CA certificates:
about 32 MB.

## Why both workflows retry

Fly's remote builder intermittently cannot reach Fly's own registry while
pushing layers. The retry deliberately builds on the GitHub runner
(`--local-only`) and pushes straight to `registry.fly.io`, so the image never
traverses the leg that is failing. It is a different path, not the same roll of
the dice — a genuine failure still fails twice.

## What the health check is for

`/health` reports the registry sizes, not a bare `ok`:

```json
{"ok":true,"languages":827,"ecosystems":31,"toolchains":1109,"registries":42}
```

A binary that somehow started with an empty registry would return 200 on every
page and serve nothing. Both workflows assert the counts are non-zero, and the
preview comment prints them — on a data PR, "827 → 831" is the diff.
