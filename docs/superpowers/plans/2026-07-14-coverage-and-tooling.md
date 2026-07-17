# rtz Coverage & Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move rtz to the house-style nightly + cargo-make + nextest + llvm-cov toolchain, scope the coverage denominator honestly, and add high-value tests so the runtime-reachable surface is well covered.

**Architecture:** Three-crate workspace (`rtz-core`, `rtz-build`, `rtz`). We (A) switch tooling to match `kord`/`conclave`, (B) mark genuinely-build-time code `#[coverage(off)]` and delete dead tarpaulin markers + dead web scaffolding, and (C) add characterization/regression tests for the `unsafe` codec, the ingestion→cache pipeline, the web helpers/error paths, and the CLI binary.

**Tech Stack:** Rust (nightly-2026-07-13), cargo-make, cargo-nextest, cargo-llvm-cov, bincode 2, axum, geo/geojson, tempfile.

## Global Constraints

- **Toolchain:** `nightly-2026-07-13` (rustc 1.99.0-nightly), pinned in `rust-toolchain.toml`. rtz is knowingly nightly-only after this plan.
- **Canonical commands:** `cargo make test`, `cargo make clippy`, `cargo make codecov`. `cargo make test` = `cargo nextest run --features web` (default `cli` feature stays on, so the `rtz` bin is built).
- **Warnings are errors:** clippy runs with `-D warnings`; the crates keep `#![warn(clippy::all)]` + `missing_docs`. Every new public item needs a doc comment.
- **Tests need features:** codec tests require `self-contained`; ingestion tests require `tz-ned`; web tests require `web`. All are satisfied under `cargo make test`.
- **`PartialEq` gotcha:** `NedTimezone` and `OsmAdmin` implement `PartialEq` on `id` only — assert other fields explicitly, never rely on `==` for value equality.
- **Commit policy (user rule):** Do **not** push. Each task ends with a commit step, but the commit runs **only after Aaron has reviewed that task's diff**. Under subagent-driven execution this is the between-task review gate.
- **Branch:** all work on `feat/coverage-and-tooling` (created in Task 1), never on `main`.
- **Spirit:** minimal, focused changes; match existing in-module test conventions; no unrelated refactors.

## File Structure

- `rust-toolchain.toml` — nightly pin (modify).
- `Makefile.toml` — cargo-make tasks (create).
- `.config/nextest.toml` — nextest profile (create).
- `.github/workflows/build.yml` — CI on cargo-make + nightly (rewrite).
- `rtz-core/Cargo.toml` — drop `tarpaulin_include` from check-cfg (modify).
- `rtz-core/src/lib.rs`, `rtz-build/src/lib.rs`, `rtz/src/lib.rs` — add `#![feature(coverage_attribute)]` (modify).
- `rtz-core/src/geo/{shared.rs,tz/ned.rs,tz/osm.rs,tz/shared.rs,admin/osm.rs,admin/shared.rs}` — remove dead tarpaulin markers; add `#[coverage(off)]` to build-time fns (modify).
- `rtz-build/src/lib.rs` — `#[coverage(off)]` on build-script fns (modify).
- `rtz/src/web/utilities.rs` — `#[coverage(off)]` on `shutdown_signal` (modify).
- `rtz/src/web/types.rs` — delete dead scaffolding; add unit tests (modify).
- `rtz-core/src/geo/shared.rs` — add `codec_tests` module (modify).
- `rtz-core/tests/ingestion.rs` — ingestion→cache pipeline test (create).
- `rtz/src/web/mod.rs` — extend the in-module web tests (modify).
- `rtz/tests/cli.rs` — CLI e2e tests (create).
- `rtz/Cargo.toml` — drop `rust-version` (modify).
- `README.md`, `DEVELOPMENT.md` — nightly reality (modify).

---

### Task 1: Switch to nightly and verify a green baseline

**Files:**
- Modify: `rust-toolchain.toml`

**Interfaces:**
- Produces: repo builds and tests pass on `nightly-2026-07-13` with the *existing* suite (no new code yet).

- [ ] **Step 1: Create the working branch**

Run:
```bash
git checkout -b feat/coverage-and-tooling
```

- [ ] **Step 2: Install the pinned nightly**

Run:
```bash
rustup toolchain install nightly-2026-07-13 --profile minimal --component llvm-tools-preview clippy rustfmt
```
Expected: toolchain installs (or "already installed").

- [ ] **Step 3: Pin the toolchain**

Replace the entire contents of `rust-toolchain.toml` with:
```toml
[toolchain]
channel = "nightly-2026-07-13"
```

- [ ] **Step 4: Verify the existing suite still builds and passes on nightly**

Run:
```bash
cargo test --features web
```
Expected: PASS — same 25 tests green (this is the pre-existing suite; run via plain `cargo test` because cargo-make/nextest arrive in Task 2). If anything fails to compile on nightly, stop and resolve before continuing (this is the derisk gate).

- [ ] **Step 5: Commit** (after review)

```bash
git add rust-toolchain.toml
git commit -m "chore: pin nightly-2026-07-13 toolchain"
```

---

### Task 2: Add cargo-make + nextest config (house style)

**Files:**
- Create: `Makefile.toml`
- Create: `.config/nextest.toml`

**Interfaces:**
- Produces: `cargo make test`, `cargo make clippy`, `cargo make codecov`, `cargo make codecov-html`, `cargo make build-{linux,windows,macos}`.

- [ ] **Step 1: Write `Makefile.toml`**

Create `Makefile.toml` with exactly:
```toml
# Tooling installers (in CI, cargo-binstall is provided by the cargo-bins action).

[tasks.install-cargo-binstall]
# No-op locally; documents the dependency. Install manually if missing:
# https://github.com/cargo-bins/cargo-binstall
script = "echo 'Checking cargo-binstall availability...'"

[tasks.install-nextest]
dependencies = ["install-cargo-binstall"]
command = "cargo"
args = ["binstall", "cargo-nextest", "--no-confirm"]

[tasks.install-llvm-cov]
dependencies = ["install-cargo-binstall"]
command = "cargo"
args = ["binstall", "cargo-llvm-cov", "--no-confirm"]

[tasks.tools]
dependencies = ["install-nextest", "install-llvm-cov"]

# Build / test.

[tasks.fmt]
workspace = false
command = "cargo"
args = ["fmt"]

[tasks.build]
workspace = false
command = "cargo"
args = ["build"]

[tasks.clippy]
workspace = false
command = "cargo"
args = ["clippy", "--all-targets", "--features", "web", "--", "-D", "warnings"]

[tasks.test]
workspace = false
dependencies = ["install-nextest"]
command = "cargo"
args = ["nextest", "run", "--features", "web"]

[tasks.codecov]
workspace = false
dependencies = ["tools"]
command = "cargo"
args = ["llvm-cov", "nextest", "--features", "web", "--workspace", "--lcov", "--output-path", "coverage.lcov"]

[tasks.codecov-html]
workspace = false
dependencies = ["tools"]
command = "cargo"
args = ["llvm-cov", "nextest", "--features", "web", "--workspace", "--html"]

# Release cross-builds (default `cli` feature stays on, so the bin is produced).

[tasks.build-linux]
workspace = false
command = "cargo"
args = ["build", "--features", "full", "--target", "x86_64-unknown-linux-gnu", "--release"]

[tasks.build-windows]
workspace = false
command = "cargo"
args = ["build", "--features", "full", "--target", "x86_64-pc-windows-gnu", "--release"]

[tasks.build-macos]
workspace = false
command = "cargo"
args = ["build", "--features", "full", "--target", "aarch64-apple-darwin", "--release"]
```

- [ ] **Step 2: Write `.config/nextest.toml`**

Create `.config/nextest.toml` with exactly:
```toml
[profile.default]
# The randomized `can_verify_lookup_assisted_accuracy` tests and the CLI e2e tests
# (which spawn the real binary) are the only mildly-flaky ones; a couple of retries
# keep a transient blip from reddening an otherwise-green run.
retries = 2
slow-timeout = { period = "30s", terminate-after = 4 }
```

- [ ] **Step 3: Verify cargo-make drives the suite**

Run:
```bash
cargo make test
```
Expected: PASS — nextest runs the existing 25 tests green.

- [ ] **Step 4: Verify clippy is clean**

Run:
```bash
cargo make clippy
```
Expected: PASS — no warnings. (If pre-existing warnings surface on nightly, fix only those that block; note anything larger for review.)

- [ ] **Step 5: Commit** (after review)

```bash
git add Makefile.toml .config/nextest.toml
git commit -m "build: add cargo-make + nextest config matching house style"
```

---

### Task 3: Rewrite CI onto cargo-make + nightly

**Files:**
- Modify: `.github/workflows/build.yml`

**Interfaces:**
- Produces: CI that runs `cargo make test` / `cargo make codecov` on nightly and cross-builds via `cargo make build-*`.

- [ ] **Step 1: Replace `.github/workflows/build.yml`**

Replace the entire file with:
```yaml
on: [push]

name: Build and Test

env:
  RUST_TOOLCHAIN: nightly-2026-07-13
  # Authenticate cargo-binstall's GitHub API calls so concurrent jobs don't
  # exhaust the unauthenticated rate limit.
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
      - uses: cargo-bins/cargo-binstall@main
      - uses: Swatinem/rust-cache@v2
        with:
          cache-all-crates: "true"
      - run: cargo binstall cargo-make --force --no-confirm
      - run: cargo make test

  codecov:
    needs: test
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          components: llvm-tools-preview
      - uses: cargo-bins/cargo-binstall@main
      - uses: Swatinem/rust-cache@v2
        with:
          cache-all-crates: "true"
      - run: cargo binstall cargo-make --force --no-confirm
      - run: cargo make codecov
      - uses: codecov/codecov-action@v5
        with:
          token: ${{ secrets.CODECOV_TOKEN }}
          slug: twitchax/rtz
          files: coverage.lcov

  build_windows:
    needs: test
    name: Build Windows
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install mingw-w64
        run: sudo apt-get update && sudo apt-get install -y mingw-w64
      - uses: dtolnay/rust-toolchain@nightly
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          targets: x86_64-pc-windows-gnu
      - uses: cargo-bins/cargo-binstall@main
      - uses: Swatinem/rust-cache@v2
      - run: cargo binstall cargo-make --force --no-confirm
      - run: cargo make build-windows
      - uses: actions/upload-artifact@v4
        with:
          name: rtz_x86_64-pc-windows-gnu
          path: target/x86_64-pc-windows-gnu/release/rtz.exe

  build_linux:
    needs: test
    name: Build Linux
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          targets: x86_64-unknown-linux-gnu
      - uses: cargo-bins/cargo-binstall@main
      - uses: Swatinem/rust-cache@v2
      - run: cargo binstall cargo-make --force --no-confirm
      - run: cargo make build-linux
      - uses: actions/upload-artifact@v4
        with:
          name: rtz_x86_64-unknown-linux-gnu
          path: target/x86_64-unknown-linux-gnu/release/rtz

  build_macos:
    needs: test
    name: Build MacOS
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          targets: aarch64-apple-darwin
      - uses: cargo-bins/cargo-binstall@main
      - uses: Swatinem/rust-cache@v2
      - run: cargo binstall cargo-make --force --no-confirm
      - run: cargo make build-macos
      - uses: actions/upload-artifact@v4
        with:
          name: rtz_aarch64-apple-darwin
          path: target/aarch64-apple-darwin/release/rtz
```

Notes for the reviewer: (1) build jobs intentionally keep the current "build on every push" behavior (no `if: main` gate) to minimize behavior change — say the word to gate them like kord. (2) `codecov-action@v5` needs a `CODECOV_TOKEN` secret; if the repo doesn't have one, fall back to v4 tokenless.

- [ ] **Step 2: Lint the YAML locally**

Run:
```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/build.yml')); print('yaml ok')"
```
Expected: `yaml ok`.

- [ ] **Step 3: Commit** (after review)

```bash
git add .github/workflows/build.yml
git commit -m "ci: run tests/coverage via cargo-make on nightly"
```

---

### Task 4: Remove dead tarpaulin markers; scope the denominator with `#[coverage(off)]`

**Files:**
- Modify: `rtz-core/Cargo.toml`
- Modify: `rtz-core/src/lib.rs`, `rtz-build/src/lib.rs`, `rtz/src/lib.rs`
- Modify: `rtz-core/src/geo/shared.rs`, `rtz-core/src/geo/tz/ned.rs`, `rtz-core/src/geo/tz/osm.rs`, `rtz-core/src/geo/tz/shared.rs`, `rtz-core/src/geo/admin/osm.rs`, `rtz-core/src/geo/admin/shared.rs`
- Modify: `rtz-build/src/lib.rs`, `rtz/src/web/utilities.rs`

**Interfaces:**
- Produces: build-time / signal-handler code excluded from coverage; no `tarpaulin_include` references remain.

- [ ] **Step 1: Delete the six dead tarpaulin markers**

In each of these files remove the line `#![cfg(not(tarpaulin_include))]` (and the 1–2 explanatory comment lines directly above it):
`rtz-core/src/geo/shared.rs`, `rtz-core/src/geo/tz/ned.rs`, `rtz-core/src/geo/tz/osm.rs`, `rtz-core/src/geo/tz/shared.rs`, `rtz-core/src/geo/admin/osm.rs`, `rtz-core/src/geo/admin/shared.rs`.

- [ ] **Step 2: Drop `tarpaulin_include` from check-cfg**

In `rtz-core/Cargo.toml`, change:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(wasm)', 'cfg(tarpaulin_include)'] }
```
to:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(wasm)'] }
```

- [ ] **Step 3: Enable the coverage attribute in the three crate roots**

At the top of each of `rtz-core/src/lib.rs`, `rtz-build/src/lib.rs`, and `rtz/src/lib.rs`, add as the first inner attribute:
```rust
#![feature(coverage_attribute)]
```
(For `rtz-build/src/lib.rs`, place it after the existing `#![cfg(not(target_family = "wasm"))]` line.)

- [ ] **Step 4: Mark the build-script body `#[coverage(off)]`**

In `rtz-build/src/lib.rs`, add `#[coverage(off)]` immediately above each `fn`: `main`, `assets_dir`, `generate_self_contained_bincodes`, `generate_ned_tz_bincodes`, `generate_osm_tz_bincodes`, `generate_osm_admin_bincodes`. Example for the first:
```rust
/// Main entry point for build script.
#[coverage(off)]
pub fn main() {
    #[cfg(feature = "self-contained")]
    generate_self_contained_bincodes();
}
```

- [ ] **Step 5: Mark the network-download sources `#[coverage(off)]`**

Add `#[coverage(off)]` immediately above the `pub fn get_geojson_features_from_source()` free function in each of `rtz-core/src/geo/tz/ned.rs`, `rtz-core/src/geo/tz/osm.rs`, `rtz-core/src/geo/admin/osm.rs`. Example:
```rust
/// Get the GeoJSON [`geojson::Feature`]s from the source.
#[cfg(not(target_family = "wasm"))]
#[coverage(off)]
pub fn get_geojson_features_from_source() -> geojson::FeatureCollection {
```

- [ ] **Step 6: Mark the build-time encode paths `#[coverage(off)]`**

In `rtz-core/src/geo/shared.rs`, add `#[coverage(off)]` immediately above these functions: `generate_lookup_bincode`, `generate_item_bincode`, `generate_bincodes`. (Leave `get_items_from_features`, `get_lookup_from_geometries`, `get_geojson_features_from_string` in the denominator — they get tested.)

- [ ] **Step 7: Mark the signal handler `#[coverage(off)]`**

In `rtz/src/web/utilities.rs`, add `#[coverage(off)]` above `pub async fn shutdown_signal()` (it blocks on OS signals and cannot be unit-tested cheaply).

- [ ] **Step 8: Verify build + tests still pass**

Run:
```bash
cargo make test
```
Expected: PASS — still 25 tests green, no `unexpected_cfg` warnings.

- [ ] **Step 9: Verify coverage runs and excludes build-time code**

Run:
```bash
cargo make codecov 2>/dev/null | tail -25
```
Expected: a coverage table; `rtz-build/src/lib.rs` and the download functions no longer drag the denominator (their lines are excluded). Note the new TOTAL for the final report.

- [ ] **Step 10: Commit** (after review)

```bash
git add -A
git commit -m "test: drop dead tarpaulin markers, scope coverage with #[coverage(off)]"
```

---

### Task 5: Delete dead web scaffolding (YAGNI)

**Files:**
- Modify: `rtz/src/web/types.rs`

**Interfaces:**
- Produces: a smaller, honest `web/types.rs` with only live helpers.

- [ ] **Step 1: Confirm which helpers are truly unused**

Run:
```bash
grep -rn "custom\|build_err\|bad_req\|internal_err\|unauthorized\|notfound\|WebResultMapper\|or_bad_req\|or_internal_err\|or_unauthorized\|or_notfound\|or_too_many_requests\|map_unauthorized\|map_too_many_requests\|map_internal_err\|map_notfound\|map_bad_req" rtz/src --include=*.rs | grep -v "src/web/types.rs"
```
Expected: **no matches** (these are referenced only within `types.rs` itself). If any symbol *does* appear elsewhere, keep that symbol and its transitive dependencies.

- [ ] **Step 2: Delete the confirmed-dead scaffolding**

In `rtz/src/web/types.rs`, delete (assuming Step 1 confirmed them unused): `custom`, `build_err`, `bad_req`, `internal_err`, `unauthorized`, `notfound`, `map_unauthorized`, `map_too_many_requests`, and the entire `WebResultMapper` trait plus both its impls (`impl ... for Result<T, E>`, `impl<T> ... for Option<T>`). Keep everything still referenced by `server.rs`/`mod.rs`: `WebError`, `WebResult`, `WebVoid`, `IfModifiedSince`, `AppState`, `get_last_modified_time`, and any `map_*` that Step 1 showed is still used (delete the rest).

- [ ] **Step 3: Verify it still compiles and passes**

Run:
```bash
cargo make test
```
Expected: PASS. If the compiler flags a now-unused import in `types.rs`, remove it.

- [ ] **Step 4: Commit** (after review)

```bash
git add rtz/src/web/types.rs
git commit -m "refactor: remove unused web error-helper scaffolding"
```

---

### Task 6: Codec roundtrip tests (the `unsafe` crown jewel)

**Files:**
- Modify: `rtz-core/src/geo/shared.rs` (add a test module at the end)

**Interfaces:**
- Consumes: `EncodableString`, `EncodableOptionString`, `EncodableIds`, `EncodableGeometry`, `get_global_bincode_config` (all `pub` in this module); `crate::base::types::Float`.
- Produces: coverage of the `Encode`/`Decode` impls and a regression guard on encode↔decode symmetry.

- [ ] **Step 1: Add the test module**

Append to `rtz-core/src/geo/shared.rs`:
```rust
#[cfg(all(test, feature = "self-contained"))]
mod codec_tests {
    use super::*;
    use crate::base::types::Float;
    use geo::{Coord, Geometry, LineString, MultiPolygon, Polygon};
    use std::borrow::Cow;

    fn roundtrip_string(s: &str) {
        let cfg = get_global_bincode_config();
        let original = EncodableString(Cow::Owned(s.to_string()));
        let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
        let (decoded, _len): (EncodableString, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded, original, "string roundtrip failed for {s:?}");
    }

    #[test]
    fn string_roundtrips_ascii_empty_nonascii_and_alignment() {
        roundtrip_string(""); // empty
        roundtrip_string("America/Los_Angeles");
        roundtrip_string("مصر"); // non-ASCII UTF-8 through pad/unpad
        roundtrip_string("abc"); // len 3 -> 1 pad byte
        roundtrip_string("abcd"); // len 4 -> full extra 4 pad bytes (alignment boundary)
    }

    #[test]
    fn option_string_roundtrips_none_and_some() {
        let cfg = get_global_bincode_config();
        for original in [EncodableOptionString(None), EncodableOptionString(Some(Cow::Owned("x".to_string())))] {
            let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
            let (decoded, _len): (EncodableOptionString, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn ids_roundtrip_empty_and_many() {
        let cfg = get_global_bincode_config();
        for v in [Vec::<Id>::new(), vec![0u32, 1, 2, 4_000_000]] {
            let original = EncodableIds(v.clone());
            let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
            let (decoded, _len): (EncodableIds, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
            assert_eq!(decoded.0, v);
        }
    }

    #[test]
    fn geometry_roundtrips_polygon_with_interior_and_multipolygon() {
        let cfg = get_global_bincode_config();
        let exterior = LineString(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 4.0, y: 0.0 },
            Coord { x: 4.0, y: 4.0 },
            Coord { x: 0.0, y: 4.0 },
            Coord { x: 0.0, y: 0.0 },
        ]);
        let interior = LineString(vec![
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 2.0, y: 1.0 },
            Coord { x: 2.0, y: 2.0 },
            Coord { x: 1.0, y: 1.0 },
        ]);
        let poly: Polygon<Float> = Polygon::new(exterior, vec![interior]);

        let original = EncodableGeometry(Geometry::Polygon(poly.clone()));
        let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
        let (decoded, _len): (EncodableGeometry, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded.0, original.0);

        let multi = EncodableGeometry(Geometry::MultiPolygon(MultiPolygon::new(vec![poly])));
        let bytes = bincode::encode_to_vec(&multi, cfg).unwrap();
        let (decoded, _len): (EncodableGeometry, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded.0, multi.0);
    }

    #[test]
    fn string_borrow_decode_matches_owned() {
        // The borrowed decode path builds a `Cow::Borrowed` via an internal transmute to
        // 'static. Exercising it here is sound because the source buffer outlives the decoded
        // value and `Cow::Borrowed` frees nothing on drop. We deliberately do NOT borrow-decode
        // the `Vec::from_raw_parts` types (EncodableIds / EncodableGeometry) from a local buffer:
        // their decoded Vecs would free borrowed memory on drop. Those borrow paths are already
        // exercised at runtime against the embedded 'static bincodes by the `geo::*` tests.
        let cfg = get_global_bincode_config();
        let original = EncodableString(Cow::Owned("America/Los_Angeles".to_string()));
        let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
        let (decoded, _len): (EncodableString, usize) = bincode::borrow_decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded.as_ref(), original.as_ref());
    }
}
```

- [ ] **Step 2: Run the codec tests**

Run:
```bash
cargo make test -- codec_tests
```
Expected: PASS (5 tests). These characterize existing behavior — a **failure is a real latent codec bug**; stop and investigate rather than "fixing" the test.

- [ ] **Step 3: Commit** (after review)

```bash
git add rtz-core/src/geo/shared.rs
git commit -m "test: add codec roundtrip coverage for the zero-copy encodables"
```

---

### Task 7: Ingestion → cache pipeline test (fixture-driven, no network)

**Files:**
- Create: `rtz-core/tests/ingestion.rs`

**Interfaces:**
- Consumes: `get_geojson_features_from_string`, `get_items_from_features`, `get_lookup_from_geometries`, `NedTimezone` (all `pub`); the fixture `test/ne_10m_time_zones.test.geojson` (3 features; feature 0 = "Arctic Ocean", `UTC-10:00`, zone `-10`).

- [ ] **Step 1: Write the integration test**

Create `rtz-core/tests/ingestion.rs`:
```rust
//! Exercises the GeoJSON → items → lookup-cache pipeline against a small committed
//! fixture, so the pure preprocessing path is covered without any network download.
#![cfg(feature = "tz-ned")]

use rtz_core::geo::{
    shared::{get_geojson_features_from_string, get_items_from_features, get_lookup_from_geometries},
    tz::ned::NedTimezone,
};

const FIXTURE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/ne_10m_time_zones.test.geojson"));

#[test]
fn builds_items_and_lookup_from_fixture() {
    let features = get_geojson_features_from_string(FIXTURE);
    let items = get_items_from_features::<NedTimezone>(features);

    // The fixture has exactly three timezone features, order-preserved.
    assert_eq!(items.len(), 3);

    let first = &items[0];
    assert_eq!(first.description.as_ref(), "Arctic Ocean");
    assert_eq!(first.offset.as_ref(), "UTC-10:00");
    assert_eq!(first.zone, -10.0);
    assert_eq!(first.raw_offset, -36_000); // round(-10 * 3600)

    // The lookup cache covers every 1x1 degree cell of the globe: 360 * 180.
    let cache = get_lookup_from_geometries(&items);
    assert_eq!(cache.len(), 64_800);

    // Every referenced id points at a real item.
    for ids in cache.values() {
        for &id in ids.iter() {
            assert!((id as usize) < items.len(), "id {id} out of range");
        }
    }
}
```

- [ ] **Step 2: Run it**

Run:
```bash
cargo make test -- builds_items_and_lookup_from_fixture
```
Expected: PASS.

- [ ] **Step 3: Commit** (after review)

```bash
git add rtz-core/tests/ingestion.rs
git commit -m "test: cover the geojson->items->lookup pipeline via fixture"
```

---

### Task 8: Web helper + server error/alias tests

**Files:**
- Modify: `rtz/src/web/types.rs` (add a test module)
- Modify: `rtz/src/web/mod.rs` (extend the existing `mod tests`)

**Interfaces:**
- Consumes: `WebError`, `get_last_modified_time` (types.rs); `get_client()`, `Body`, `Request`, `StatusCode`, `BodyExt`, `ServiceExt::oneshot` (mod.rs test module).

- [ ] **Step 1: Add unit tests to `web/types.rs`**

Append to `rtz/src/web/types.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn web_error_display_is_the_message() {
        let e = WebError { status: 400, message: "boom".to_string(), backtrace: None };
        assert_eq!(e.to_string(), "boom");
    }

    #[test]
    fn web_error_into_response_uses_its_status() {
        let e = WebError { status: 404, message: "nope".to_string(), backtrace: None };
        let response = e.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn last_modified_time_is_nonempty() {
        assert!(!get_last_modified_time().is_empty());
    }
}
```

- [ ] **Step 2: Extend the `web/mod.rs` test module**

Add these tests inside the existing `mod tests` in `rtz/src/web/mod.rs` (the `use` lines there already import `Body`, `Request`, `StatusCode`, `BodyExt`, `ServiceExt`):
```rust
    #[tokio::test]
    async fn unversioned_ned_matches_v1_body() {
        let client = get_client();

        let a = client.clone().oneshot(Request::get("/api/ned/tz/-121.0/46.0").body(Body::empty()).unwrap()).await.unwrap();
        let a_body = a.into_body().collect().await.unwrap().to_bytes();

        let b = client.oneshot(Request::get("/api/v1/ned/tz/-121.0/46.0").body(Body::empty()).unwrap()).await.unwrap();
        let b_body = b.into_body().collect().await.unwrap().to_bytes();

        assert_eq!(a_body, b_body);
    }

    #[tokio::test]
    async fn malformed_coordinate_is_bad_request() {
        let client = get_client();

        let request = Request::get("/api/v1/ned/tz/not-a-number/46.0").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn unknown_route_is_not_found() {
        let client = get_client();

        let request = Request::get("/api/does-not-exist").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn health_endpoint_is_ok() {
        let client = get_client();

        let request = Request::get("/api/health").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
```

- [ ] **Step 3: Run the web tests**

Run:
```bash
cargo make test -- web::
```
Expected: PASS (original 5 web tests + 4 new mod.rs tests + 3 new types.rs tests).

- [ ] **Step 4: Commit** (after review)

```bash
git add rtz/src/web/types.rs rtz/src/web/mod.rs
git commit -m "test: cover web error helpers, route aliases, and error statuses"
```

---

### Task 9: CLI end-to-end tests (spawn the real binary)

**Files:**
- Create: `rtz/tests/cli.rs`

**Interfaces:**
- Consumes: `env!("CARGO_BIN_EXE_rtz")` (Cargo provides this to integration tests).

Note: no `dump-geojson` test — under `--features web` it would serialize the entire OSM admin
dataset (~46 MB bincode) to GeoJSON on disk, far too heavy for a routine test. The four tests below
cover arg parsing, resolution output, the error path, and `--version` without that cost.

- [ ] **Step 1: Write the CLI e2e tests**

Create `rtz/tests/cli.rs`:
```rust
//! End-to-end tests that spawn the real `rtz` binary, per the repo convention for
//! binaries (`CARGO_BIN_EXE_<name>`).

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rtz");

#[test]
fn ned_tz_resolves_a_known_point() {
    let output = Command::new(BIN).args(["ned", "tz", "-87.62,41.88"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("America/Chicago"), "stdout was: {stdout}");
}

#[test]
fn osm_admin_resolves_a_known_point() {
    let output = Command::new(BIN).args(["osm", "admin", "30,30"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Name:"), "stdout was: {stdout}");
}

#[test]
fn malformed_lng_lat_exits_nonzero() {
    let output = Command::new(BIN).args(["ned", "tz", "not-a-coordinate"]).output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn version_flag_prints_the_crate_version() {
    let output = Command::new(BIN).arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "stdout was: {stdout}");
}
```

- [ ] **Step 2: Run the CLI tests**

Run:
```bash
cargo make test -- --test cli
```
Expected: PASS (4 tests). These spawn the real binary, so they are slower; that's expected.

- [ ] **Step 3: Commit** (after review — SEE EXECUTION OVERRIDE: no commit; stage only)

```bash
git add rtz/tests/cli.rs
```

---

### Task 10: Update docs to the nightly reality

**Files:**
- Modify: `rtz/Cargo.toml`, `README.md`, `DEVELOPMENT.md`

**Interfaces:**
- Produces: docs/metadata that no longer claim stable-install / MSRV.

- [ ] **Step 1: Drop the MSRV metadata from `rtz/Cargo.toml`**

Remove the line `rust-version = "1.80"` from the `[package]` table of `rtz/Cargo.toml`.

- [ ] **Step 2: Fix the README**

In `README.md`:
- Delete the MSRV badge line: `[![Rust](https://img.shields.io/crates/msrv/rtz)](https://github.com/twitchax/rtz)`.
- Under `### Install`, change the Cargo instructions to require nightly:
```bash
$ cargo +nightly install rtz
```
- Immediately under the `## Binary Usage` heading (before `### Install`), add:
```markdown
> **Note:** Building rtz from source requires a **nightly** Rust toolchain (pinned in
> `rust-toolchain.toml`). Pre-built release binaries below need no toolchain.
```

- [ ] **Step 3: Note nightly in `DEVELOPMENT.md`**

At the top of `DEVELOPMENT.md` (after the `# rtz development` heading), add:
```markdown
> Requires the nightly toolchain pinned in `rust-toolchain.toml`. Use `cargo make test`,
> `cargo make clippy`, and `cargo make codecov`.
```

- [ ] **Step 4: Sanity-check the crate still packages**

Run:
```bash
cargo package -p rtz --no-verify --allow-dirty >/dev/null && echo "package ok"
```
Expected: `package ok` (metadata is valid without `rust-version`).

- [ ] **Step 5: Commit** (after review)

```bash
git add rtz/Cargo.toml README.md DEVELOPMENT.md
git commit -m "docs: reflect the nightly-only build requirement"
```

---

### Task 11: Final verification and honest coverage report

**Files:** none (verification only).

- [ ] **Step 1: Full clean run of the house-style tasks**

Run:
```bash
cargo make clippy && cargo make test
```
Expected: both PASS, zero warnings.

- [ ] **Step 2: Produce the scoped coverage number**

Run:
```bash
cargo make codecov 2>/dev/null | tail -30
```
Record the per-file table and TOTAL. Confirm: no `rtz-build`/download lines dragging the denominator; `rtz-core` codec + pipeline meaningfully up from the 15–27% baseline; `web/*` up from the baseline; `web/utilities.rs` now `#[coverage(off)]` (absent) rather than 0%.

- [ ] **Step 3: Confirm no tarpaulin residue**

Run:
```bash
grep -rn "tarpaulin" . --include=*.rs --include=*.toml | grep -v target || echo "no tarpaulin references"
```
Expected: `no tarpaulin references`.

- [ ] **Step 4: Report against acceptance criteria**

Summarize for Aaron: final honest line-coverage on the scoped denominator (target ≥ ~85% of runtime-reachable code, reported not gamed), the new test count, and that CI/tooling now mirror the house style. Do not merge — hand back for review.

---

## Self-Review

**Spec coverage:**
- Part A tooling → Tasks 1 (nightly), 2 (Makefile/nextest), 3 (CI), 4 Step 1–2 (delete markers). ✓
- Part B scoping → Task 4 (coverage_attribute + `#[coverage(off)]`). ✓
- Part C tests → Task 6 (codec), 7 (ingestion), 8 (web), 9 (CLI), 5 (delete dead scaffolding). ✓
- Consequence (MSRV/README/Cargo/DEVELOPMENT) → Task 10. ✓
- WASM defer → no wasm task; `wasm.rs` is `#[cfg(feature="wasm")]` and outside the `web` coverage build. ✓
- Acceptance criteria → Task 11. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code and exact commands. ✓

**Type consistency:** `get_global_bincode_config` (Copy config) passed by value; `EncodableString`/`EncodableOptionString` compared via derived `PartialEq`; `EncodableIds`/`EncodableGeometry` compared via `.0` (no `PartialEq` derive); `NedTimezone` fields asserted individually (its `PartialEq` is id-only); `get_items_from_features::<NedTimezone>` matches the `pub fn` signature; `env!("CARGO_BIN_EXE_rtz")` matches the `[[bin]] name = "rtz"`. ✓
