[![Build and Test](https://github.com/twitchax/rtz/actions/workflows/build.yml/badge.svg)](https://github.com/twitchax/rtz/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/twitchax/rtz/branch/main/graph/badge.svg?token=35MZN0YFZF)](https://codecov.io/gh/twitchax/rtz)
[![Version](https://img.shields.io/crates/v/rtz.svg)](https://crates.io/crates/rtz)
[![Crates.io](https://img.shields.io/crates/d/rtz?label=crate)](https://crates.io/crates/rtz)
[![GitHub all releases](https://img.shields.io/github/downloads/twitchax/kord/total?label=binary)](https://github.com/twitchax/rtz/releases)
[![npm](https://img.shields.io/npm/dt/rtzweb?label=npm)](https://www.npmjs.com/package/rtzweb)
[![Documentation](https://docs.rs/rtz/badge.svg)](https://docs.rs/rtz)
[![Rust](https://img.shields.io/badge/rust-nightly-blue.svg?maxAge=3600)](https://github.com/twitchax/rtz)
[![License:MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# rtz

A self-contained timezone library / binary / server for Rust / JS (via WASM) ([free server](https://tz.twitchax.com/)).

## Binary Usage

### Install

Cargo:

```bash
$ cargo install rtz
```

NPM:

```bash
$ npm install --save rtzweb
```

### Wasmer

### Help Docs

```bash

```

## Library Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rtz = "*" #choose a version
```

### Examples



## JS Usage



## Feature Flags

The library and binary both support various feature flags.  Of most important note are:
* `default = ["cli"]`
* `cli`: enables the CLI features, and can be removed if only compiling the library.
* `server`: enables the `serve` subcommand, which starts a Rocket web server that can respond to time zone requests.

## Test

```bash
cargo test
```

## Bench

```bash
cargo bench
```

## License

MIT