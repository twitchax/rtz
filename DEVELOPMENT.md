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
$ cargo wasi build --release --no-default-features --features cli
$ wasmer publish
```

## Publish to Docker

```bash
docker build -t twitchax/rtz:0.3.0 -f ./docker/Dockerfile .
```

```bash
docker push twitchax/rtz:0.3.0
```