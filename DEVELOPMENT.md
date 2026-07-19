# rtz development

> Builds and tests on the **stable** toolchain pinned in `rust-toolchain.toml`. Use `cargo make
> test`, `cargo make clippy`, and `cargo make codecov`. Coverage is the one exception: it needs a
> **nightly** toolchain (with `llvm-tools-preview`) for the `#[coverage(off)]` exclusions, so
> `cargo make codecov` invokes `cargo +nightly llvm-cov` for you.

## Full Tests

```bash
$ cargo make test
```

## Publish to Cargo

Make sure `rtz` references the correct versions of `rtz-core` and `rtz-build` in `Cargo.toml`.

```bash
$ cargo publish -p rtz-core
$ cargo publish -p rtz-build
$ cargo publish -p rtz
```

## Benchmarks

```bash
$ cargo bench --features web
```

## Publish to NPM

```bash
$ cd rtz
$ wasm-pack build --target web --no-default-features --features tz-osm --features tz-ned --features self-contained --features wasm --features extrasimplified
```

Rename package to `rtzweb` in `pkg/package.json`.

```bash
$ wasm-pack publish
```

## Publish to wasmer

```bash
$ # Build the WASI binary. (`cargo-wasi`'s old `wasm32-wasi` target is unsupported on modern
$ # toolchains; use `wasm32-wasip1` directly. Bump the version in `wasmer.toml` first.)
$ rustup target add wasm32-wasip1
$ cargo build --release --features full --target wasm32-wasip1
$ wasmer publish
```

## Publish to Docker

```bash
$ docker build -f ./docker/Dockerfile -t twitchax/rtz:{v} .
```

```bash
$ docker push twitchax/rtz:{v}
```