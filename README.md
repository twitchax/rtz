[![Build and Test](https://github.com/twitchax/rtz/actions/workflows/build.yml/badge.svg)](https://github.com/twitchax/rtz/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/twitchax/rtz/branch/main/graph/badge.svg?token=35MZN0YFZF)](https://codecov.io/gh/twitchax/rtz)
[![Version](https://img.shields.io/crates/v/rtz.svg)](https://crates.io/crates/rtz)
[![Crates.io](https://img.shields.io/crates/d/rtz?label=crate)](https://crates.io/crates/rtz)
[![GitHub all releases](https://img.shields.io/github/downloads/twitchax/rtz/total?label=binary)](https://github.com/twitchax/rtz/releases)
[![npm](https://img.shields.io/npm/dt/rtzweb?label=npm)](https://www.npmjs.com/package/rtzweb)
[![Documentation](https://docs.rs/rtz/badge.svg)](https://docs.rs/rtz)
[![Rust](https://img.shields.io/badge/rust-nightly-blue.svg?maxAge=3600)](https://github.com/twitchax/rtz)
[![License:MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# rtz

A self-contained geo lookup library / binary / server for Rust / JS (via WASM) ([free server](http://tz.twitchax.com/api/v1/osm/tz/30/30)) using data from the [Natural Earth](https://www.naturalearthdata.com/) and [OpenStreetMap](https://www.openstreetmap.org/) datasets.

## Free Server

The server is deployed to four regions across the globe, and is available at [tz.twitchax.com](http://tz.twitchax.com/api/v1/osm/tz/30/30).  Each region is currently 
capable of supporting around 8,000 RPS, and is deployed to the following regions: sea, iad, ams, hkg.

In addition, the server will now generally attempt to not break backwards compatibility within an api version.  This means that the server will (attempt to) not change the response format for a given api version, and will (attempt to) not remove any fields from the response.  This does not mean that the server will not add fields to the response, but it will (attempt to) not remove them.

Requests take the form of `http://tz.twitchax.com/api/v1/osm/tz/{lng}/{lat}`.  You can also check out the [swagger docs](http://tz.twitchax.com/app-docs) to explore other endpoints and versioning strategy.

Example request:

```bash
$ curl http://tz.twitchax.com/api/v1/osm/tz/30/30

[{"id":12,"identifier":"Africa/Cairo","shortIdentifier":"EEST","offset":"UTC+03:00","rawOffset":10800,"rawBaseOffset":7200,"rawDstOffset":3600,"zone":3.0,"currentTime":"2023-07-25T23:39:59.385469400+03:00"}]
```

HTTPS is also available, but is not recommended due to the performance overhead for the client and the server, and the lack of sensitive data being transmitted.

## Binary Usage

### Install

Windows:

```powershell
$ iwr https://github.com/twitchax/rtz/releases/latest/download/rtz_x86_64-pc-windows-gnu.zip
$ Expand-Archive rtz_x86_64-pc-windows-gnu.zip -DestinationPath C:\Users\%USERNAME%\AppData\Local\Programs\rtz
```

Mac OS (Apple Silicon):

```bash
$ curl -LO https://github.com/twitchax/rtz/releases/latest/download/rtz_aarch64-apple-darwin.zip
$ unzip rtz_aarch64-apple-darwin.zip -d /usr/local/bin
$ chmod a+x /usr/local/bin/rtz
```

Linux:

```bash
$ curl -LO https://github.com/twitchax/rtz/releases/latest/download/rtz_x86_64-unknown-linux-gnu.zip
$ unzip rtz_x86_64-unknown-linux-gnu.zip -d /usr/local/bin
$ chmod a+x /usr/local/bin/rtz
```

Cargo:

```bash
$ cargo install rtz
```

NPM:

```bash
$ npm install --save rtzweb
```

### Help Docs

```bash
$ rtz

A tool to easily work with geo lookups via a binary, a library, or a server.

Usage: rtz [COMMAND]

Commands:
  ned           The Natural Earth Data dataset based operations
  osm
  dump-geojson  Resolve a timezone from a lng,lat pair using the OSM dataset
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Resolve a Time Zone

```bash
$ rtz resolve-ned "-87.62,41.88"

Identifier:      America/Chicago
UTC Offset:      UTC-06:00
Offset Seconds:  -21600
Description:     Canada (almost all of Saskatchewan), Costa Rica, El Salvador, Ecuador (Galapagos Islands), Guatemala, Honduras, Mexico (most), Nicaragua,
DST Description: Canada (Manitoba), United States (Illinois, most of Texas)
```

### Run with Wasmer

```bash
wasmer run twitchax/rtz@latest -- resolve-ned 30,30
```

### Run the Server

```bash
$ cargo install rtz --features web
$ rtz serve
```

```bash
$ docker run -it --rm -p 8082 twitchax/rtx
```

## Library Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rtz = "*" #choose a version
```

### Examples

```rust
use rtzlib::NedTimezone;
use rtzlib::CanPerformGeoLookup;

// Query a time zone for a given `(lng,lat)`.
assert_eq!(
    NedTimezone::lookup(-121., 46.)[0]
        .identifier
        .as_ref()
        .unwrap(),
    "America/Los_Angeles"
);
```

## JS Usage

The npm package is available [here](https://www.npmjs.com/package/rtzweb).

First, load the module as you would any other ES module.

```js
import * as rtz from 'rtzweb/rtzlib.js';
```

Then, you can use the library similarly as you would in Rust.

```js
let tz = rtz.getTimezoneNed(-121, 46);
tz.identifier; // "America/Los_Angeles"
```

## Feature Flags

The library and binary both support various feature flags.  Of most important note are:
* Top-Level:
  * `default = ["cli"]`
  * `full = ["tz-ned", "self-contained"]`
* Datasets:
  * `tz-ned`: enables the [Natural Earth](https://www.naturalearthdata.com/) time zone dataset, and the associated produced library functions.
  * `tz-osm`: enables the [OpenStreetMap](https://www.openstreetmap.org/) time zone dataset, and the associated produced library functions.
* Binary configuration:
  * `cli`: enables the CLI features, and can be removed if only compiling the library.
  * `self-contained`: enables the self-contained features, which build with datasets embedded into the binary.
  * `double-precision`: uses `f64`s every for `Geometry` and `Polygon` data types, which is more accurate but fatter than `f32`s.
  * `unsimplified`: produces unsimplified data caches.  Requires more binary / memory overhead, but is more accurate.  Uses the level of detail from the original dataset.  The default is to simplify to an epsilon of `0.0001`.
  * `extrasimplified`: produces extrasimplified data caches.  Requires less binary / memory overhead, but is less accurate.  This sets the simplification epsilon to `0.01`.
  * `owned-decode`: uses `owned` instead of `borrow` for the `decode` feature of the `decode` crate.  This increases memory footprint by not mapping the data directly from the binary, but is less `unsafe`-y / dark arts-y.
* Special Modifiers:
  * `wasm`: enables the WASM features, and is required to build an NPM package via `wasm-pack`.
  * `web = ["full", "borrow-decode"]`: enables the `serve` subcommand, which starts a Rocket web server that can respond to time zone requests.
* Other Considerations:
  * `wasm` / `wasi` builds currently do not play nice with `reqwest` and `zip`, so the `wasm` / `wasi` builds require the `self-contained` feature.

## Performance

### General

This implementation trades binary size for performance by employing an in-binary cache that improves average timezone resolution by about 96x, and worst-case resolution by about 10x.  The cache is generated from the [Natural Earth](https://www.naturalearthdata.com/) dataset, and is included in the binary.  The cache is generated by the `generate` subcommand, and is stored in the `assets` directory.

![Bench](static/bench.png)

### Free Server

Below is the sample performance to resolve a time zone from a `(lng,lat)` pair to one of the data centers using a concurrency of 1,000, achieving 8,000 RPS.

![Drill Perf 1](static/perf1.png)

Below is the sample performance to resolve a time zone from a `(lng,lat)` pair to one of the data centers using a concurrency of 100, achieving an average response time of `24 ms`.

![Drill Perf 2](static/perf2.png)

## Test

```bash
cargo test --features full --features web
```

## Bench

```bash
cargo bench --features full --features web
```

## License

MIT