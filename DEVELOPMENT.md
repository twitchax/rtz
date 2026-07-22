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

`cargo make wasm` runs `wasm-pack` with the right feature set and applies the `rtzweb` rename.
The rename has to happen here rather than by hand: `wasm-pack` regenerates `pkg/package.json`
from `Cargo.toml` on every build, so a manual edit silently reverts and you publish as `rtz`.

```bash
$ cargo make wasm
$ cargo make test-wasm   # exercises the JS ABI under Node
$ cd rtz/pkg && npm publish --otp={code}
```

`admin-osm` is deliberately excluded from the NPM build — it roughly triples the package size.

## Publish the WASI component

Each release attaches a WASI Preview 2 component to the GitHub release — no registry involved.

```bash
$ rustup target add wasm32-wasip2
$ cargo build --release --features full --target wasm32-wasip2
$ # Verify it actually runs before attaching it.
$ wasmtime run target/wasm32-wasip2/release/rtz.wasm ned tz 30,30
$ gh release upload v{v} target/wasm32-wasip2/release/rtz.wasm#rtz-wasm32-wasip2.wasm
```

> **wasmer is retired as of `0.8.0`.** Its runtime can't execute Preview 2 components, which pinned
> us to the legacy `wasm32-wasi`/Preview 1 target. The published `twitchax/rtz` wasmer package is
> left in place at `0.8.0` so existing invocations keep working, but new versions are not pushed
> there; `wasmer.toml` was removed (recoverable from git history if it's ever needed again).

## Publish to Docker

```bash
$ docker build -f ./docker/Dockerfile -t twitchax/rtz:{v} .
```

```bash
$ docker push twitchax/rtz:{v}
```