# rtz development

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
$ # Regenerate the cache.
$ cargo build --release --no-default-features --features full
$ cargo wasix build --release --features full
$ # Need `--no-validate` for some reason when pushing a WASIX binary.
$ wasmer publish --no-validate
```

## Publish to Docker

```bash
$ docker build -f ./docker/Dockerfile -t twitchax/rtz:{v} .
```

```bash
$ docker push twitchax/rtz:{v}
```