# langbank.dev as a container, for Fly (production and PR previews).
#
# The site has no database, no volume and no runtime data directory: every page
# is rendered from `&'static` tables that `build.rs` produced at compile time.
# So the runtime image is a static-ish binary and CA certificates, and a machine
# that boots is a machine that is fully working — there is no data to be missing.
#
# Built with cargo-chef so the dependency compile is its OWN layer, keyed on the
# lockfile rather than the source. Without it every push recompiles axum, tokio
# and hyper from scratch, and concurrent PR deploys queue behind each other on
# Fly's shared builder. With it, a push that only changes a page reuses the
# cooked layer.

FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked --version ^0.1
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
COPY --from=planner /src/recipe.json recipe.json
# Cook only the dependencies — this layer caches until the lockfile moves.
RUN cargo chef cook --release --recipe-path recipe.json -p langbank-web
# Then the workspace, which is all that recompiles on a normal push. `data/` is
# an input to build.rs, so a TOML-only change correctly invalidates from here.
COPY . .
RUN cargo build --release -p langbank-web

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/langbank-web /usr/local/bin/langbank-web

# Fly routes to internal_port 8080. Nothing else is configurable, deliberately:
# a second source of truth for what the site serves is exactly what langbank
# exists to avoid.
ENV PORT=8080
EXPOSE 8080
CMD ["langbank-web"]
