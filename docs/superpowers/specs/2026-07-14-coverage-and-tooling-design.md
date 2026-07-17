# rtz — Coverage & Tooling Design (Goal 1)

**Date:** 2026-07-14
**Status:** Approved (design), spec pending user review
**Scope:** Goal 1 of 3 in the current work stream. Goals 2 (dependency bumps) and 3
(data-ingestion automation) get their own specs. Order: **Coverage → Deps → Data.**

## Guiding principle (goal zero)

Keep the spirit of the repo intact: lean, self-contained, readable, "just works." Every change
below is weighed against that. The one place it bends — going nightly-only — is a conscious,
user-approved trade documented under *Consequences*.

## Problem / current state

CI moved from `cargo tarpaulin` to `cargo llvm-cov`, but the codebase still carries six dead
`#![cfg(not(tarpaulin_include))]` markers that llvm-cov ignores. As a result the reported number is
misleading and the build-time data-preprocessing code is fully counted against the denominator.

Measured baseline (`cargo llvm-cov nextest --workspace --features web`, lines): **47.36% total**,
which splits into two very different populations:

| Layer | Coverage | Interpretation |
|---|---|---|
| `rtz` runtime lookup (`geo/tz/*`, `geo/admin/*`, `shared`) | 86–100% | Hot path already excellent |
| `rtz-build/src/lib.rs` | 0% | Build script — never runs under tests |
| `rtz-core/*` (codec + ingestion) | 15–27% | Build-time preprocessing + the `unsafe` codec |
| `rtz/src/web/utilities.rs` | 0% | 18 lines, trivial |
| `rtz/src/web/types.rs` | 16.7% | 105 missed — mostly `#[allow(dead_code)]` scaffolding |
| `rtz/src/web/server.rs` | 57% | Error/404/alias paths |
| `rtz/src/bin.rs` | 57% | CLI branches not driven as a process |

The code users actually hit is ~90%+; the number is dragged down by build-time code plus a few
genuinely-thin spots and dead helpers.

## Decisions (locked)

1. **Sequence:** Coverage → Deps → Data.
2. **Denominator:** scope it honestly (exclude genuinely-untestable-cheaply build-time code) rather
   than chase a vanity number.
3. **WASM (`wasm.rs`):** defer/exclude. It is `#[cfg(feature = "wasm")]` and not compiled under the
   `web`-feature coverage run, so it is already outside the denominator; no wasm test runner.
4. **Scoping mechanism:** match the house idiom exactly — full nightly, `#![feature(coverage_attribute)]`,
   `#[coverage(off)]`. rtz becomes nightly-only.
5. **Nightly pin:** `nightly-2026-07-13` (latest available; rustc 1.99.0-nightly), *not* the older
   `2025-12-22` that kord/conclave pin.

## Design

### Part A — Tooling alignment (match kord/conclave house style)

- `rust-toolchain.toml` → `channel = "nightly-2026-07-13"`.
- New **`Makefile.toml`** (cargo-make), mirroring kord:
  - `install-nextest`, `install-llvm-cov`, `install-cargo-binstall` (doc no-op), `tools` aggregate.
  - `fmt` (`cargo fmt`), `build`, `clippy` (`cargo clippy --features web --all-targets`).
  - `test` → `cargo nextest run --features web`.
  - `codecov` → `cargo llvm-cov nextest --features web --workspace --lcov --output-path coverage.lcov`.
  - `codecov-html` → same with `--html`.
  - `build-linux` / `build-windows` / `build-macos` — mirror today's cross-target release builds.
- New **`.config/nextest.toml`**: `[profile.default]` with `retries = 2` and
  `slow-timeout = { period = "30s", terminate-after = 4 }`. The randomized
  `can_verify_lookup_assisted_accuracy` tests and the oneshot suite are the only mildly-flaky ones;
  no test-groups needed yet (no sockets).
- Rewrite **`.github/workflows/build.yml`** to kord's shape: `RUST_TOOLCHAIN: nightly-2026-07-13`,
  `dtolnay/rust-toolchain@nightly`, `cargo binstall cargo-make --force --no-confirm`, then
  `cargo make test` / `cargo make codecov` (→ `codecov/codecov-action@v5`, slug `twitchax/rtz`),
  and `cargo make build-*` jobs gated `if: github.ref == 'refs/heads/main'`.
- **Delete** all six `#![cfg(not(tarpaulin_include))]` markers (rtz-core: `geo/shared.rs`,
  `geo/tz/ned.rs`, `geo/tz/osm.rs`, `geo/tz/shared.rs`, `geo/admin/osm.rs`, `geo/admin/shared.rs`).

### Part B — Denominator scoping (`#[coverage(off)]`)

Add `#![feature(coverage_attribute)]` to each crate root that needs it and mark build-time /
untestable-cheaply code `#[coverage(off)]`:

- `rtz-build/src/lib.rs` — entire body (build-script only).
- `rtz-core` `get_geojson_features_from_source` in `geo/tz/ned.rs`, `geo/tz/osm.rs`,
  `geo/admin/osm.rs` (reqwest downloads of huge datasets).
- `rtz-core` `geo/shared.rs` encode/generate paths: `generate_item_bincode`,
  `generate_lookup_bincode`, `generate_bincodes` (only run during asset generation).

The runtime `borrow_decode` path, the pure transforms (`get_items_from_features`,
`get_lookup_from_geometries`), and the DTO conversions stay **in** the denominator and get tested.

### Part C — New tests (the coverage lift, ranked by value)

1. **Codec roundtrip** (rtz-core) — highest value; guards the `unsafe` zero-copy `borrow_decode`.
   Encode → decode → assert-equal for:
   - `EncodableString`: empty, ASCII, non-ASCII (Arabic, matching real admin names), and lengths on
     either side of the `Float` alignment boundary (exercises pad/unpad).
   - `EncodableOptionString`: `None` and `Some`.
   - `EncodableGeometry`: `Polygon` with interior rings, and `MultiPolygon`.
   - `EncodableIds`: empty and multi-element.
   Cover both the owned `Decode` and borrowed `BorrowDecode` paths.
2. **Ingestion → cache pipeline** (rtz-core) driven by the existing
   `test/ne_10m_time_zones.test.geojson` fixture — `get_geojson_features_from_string` →
   `get_items_from_features` → `get_lookup_from_geometries`; assert item count and a known cell's ids.
   No network.
3. **Web helpers** — `web/utilities.rs::shutdown_signal` (drive at least the setup path) and the
   *live* `web/types.rs` helpers: `IfModifiedSince` extractor, `WebError` `into_response`/`Display`,
   `get_last_modified_time`, `map_bad_req`.
4. **`web/server.rs`** — extend the in-module `oneshot` tests: malformed coordinate → status,
   unknown route → 404, unversioned route body ≡ `/v1/` route body, `/health`.
5. **CLI e2e** — new `rtz/tests/cli.rs` spawning `env!("CARGO_BIN_EXE_rtz")` in a `tempfile::TempDir`
   (matches the repo-convention for binaries): `ned tz`, `osm tz`, `osm admin`, malformed `lng,lat`
   → non-zero exit, `dump-geojson` writes files, `--version`. The two in-process `bin.rs` tests may
   remain or be folded in.
6. **Delete dead web scaffolding** rather than test/exclude it (YAGNI): the unused
   `#[allow(dead_code)]` error-builder zoo in `web/types.rs` (`custom`, `build_err`, `bad_req`,
   `internal_err`, `unauthorized`, `notfound`) and the unused `WebResultMapper` methods /
   `map_unauthorized` / `map_too_many_requests`. These are `pub(crate)` inside the `web` module —
   not public API — so removal is safe. Keep whatever `server.rs` actually calls.

## Consequences (the "spirit" trade)

Going nightly makes rtz **nightly-only to build**, so `cargo install rtz` on stable and the MSRV
badge / `rust-version = "1.80"` become false. As part of this goal, update to match reality:

- `rtz/Cargo.toml`: adjust/remove `rust-version` (and the `docs.rs`/MSRV assumptions).
- `README.md`: fix the MSRV badge and any "stable install" framing.
- `DEVELOPMENT.md`: note the nightly requirement.

This is a real narrowing of who can build rtz, accepted knowingly.

## Acceptance criteria

- `cargo make test` and `cargo make clippy` pass clean on `nightly-2026-07-13` (warnings-as-errors).
- `cargo make codecov` runs; the six tarpaulin markers are gone; build-time code is `#[coverage(off)]`.
- New tests exist and pass for: codec roundtrip, ingestion→cache, web helpers, server error/alias
  paths, CLI e2e.
- Honest line coverage of the *runtime-reachable* surface is meaningfully up (target ≥ ~85% on the
  scoped denominator; exact number reported after implementation, not gamed).
- README/Cargo.toml/DEVELOPMENT reflect the nightly reality.
- CI mirrors the kord job structure and is green.

## Out of scope (own specs later)

- Dependency bumps incl. the `geo` 0.28/0.29 skew (Goal 2).
- Data-ingestion automation + de-hardcoding the admin path (Goal 3).
- A wasm-bindgen test runner.
- Bumping clippy to `pedantic`/`nursery` (note only; not part of coverage).

## Risks

- rtz may hit a compile/lint issue on `nightly-2026-07-13` that stable 1.93 didn't surface —
  discovered on first `cargo make test`; resolve before proceeding.
- `web/types.rs` derive glue (utoipa `ToSchema`, serde) may still read as uncovered; `#[coverage(off)]`
  or a targeted serialization test as needed.
- Deleting dead scaffolding could reveal an unexpected reference; compile catches it.
