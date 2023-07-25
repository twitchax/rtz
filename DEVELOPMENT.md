# rtz development

## Publish to Cargo

```bash
$ cargo publish
```

## Publish to NPM

```bash
$ cd rtz
$ wasm-pack build --target web --no-default-features --features full --features wasm
```

Rename package to `rtzweb` in `pkg/package.json`.

```bash
$ wasm-pack publish
```

## Publish to wasmer

```bash
$ # Regenerate the cache.
$ cargo build --release --no-default-features --features full
$ cargo wasix build --release --no-default-features --features full --features cli
$ wasmer publish
```

## Publish to Docker

```bash
docker build -t twitchax/rtz:0.3.0 -f ./docker/Dockerfile .
```

```bash
docker push twitchax/rtz:0.3.0
```