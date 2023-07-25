# rtz development

## Publish to Cargo

```bash
$ cargo publish -p rtz-core
$ cargo publish -p rtz-build
$ cargo publish -p rtz
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
docker build -t twitchax/rtz:{v} -f ./docker/Dockerfile .
```

```bash
docker push twitchax/rtz:{v}
```