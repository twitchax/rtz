# syntax=docker/dockerfile:1

# Chef base — copy the toolchain file early so rustup installs the repo's pinned toolchain for every stage.
FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /build
COPY rust-toolchain.toml .

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
# Compiled dependencies are cached in their own layer, independent of source changes.
RUN cargo chef cook --release --features web --recipe-path recipe.json
# The bincode assets ship in rtz/assets, so this build is self-contained (no downloads).
COPY . .
RUN cargo build --features web --release

# Minimal runtime image.
FROM debian:stable-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 ca-certificates tini && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/rtz /rtz
EXPOSE 8082
# tini as PID 1 so SIGTERM reaches the app and its graceful-shutdown handler fires.
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/rtz", "serve"]
