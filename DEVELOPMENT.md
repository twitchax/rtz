# rtz development

## Publish to Cargo

```bash
$ cargo publish
```

## Publish to NPM

```bash
$ wasm-pack build --features wasm
```

Rename package to `rtzweb` in `pkg/package.json`.

```bash
$ wasm-pack publish
```

## Publish to wasmer

```bash
$ cargo wasi build --release --no-default-features --features cli --features web
$ cargo wasix publish
```