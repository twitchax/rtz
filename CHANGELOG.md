# Changelog

All notable changes to this project are documented here. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

A tooling and coverage pass, a dependency sweep, and a new pipeline for refreshing the embedded
datasets on demand. No committed data changed as part of this release; the pipeline exists so it
*can* be refreshed, not because it was.

### Added

- **`cargo xtask` data-update pipeline.** New `xtask` crate with `download-pbf`, `extract-admin`,
  `regen`, `verify`, `update`, and `clean` subcommands that chain them end to end: resumable
  planet-PBF download, admin-boundary extraction via `osm_extract_polygon`, a full bincode regen
  against the latest NED/OSM sources, and a decode-check via `cargo nextest`. Large artifacts (the
  ~80GB PBF, extracted GeoJSON) default into a gitignored `.rtz-data/` scratch dir, and `cargo
  xtask clean` reclaims it. Paired with a repo-local `update-data` project skill
  (`.claude/skills/update-data/SKILL.md`) that walks an operator through it, including the
  `--admin-dirs` handshake.
- **cargo-make/nextest/llvm-cov tooling**, replacing tarpaulin. `Makefile.toml` and
  `.config/nextest.toml` give us `cargo make test`, `cargo make clippy`, and `cargo make codecov`
  as the canonical commands, matching the rest of the house. Building, testing, and `cargo install
  rtz` all stay on **stable**; only `cargo make codecov` reaches for nightly (see below).
- **New test coverage**: codec-roundtrip tests for the `unsafe` zero-copy `Encode`/`Decode` impls,
  a fixture-driven ingestion test for the GeoJSON-to-lookup-cache pipeline, web error/route-alias
  tests, and CLI end-to-end tests that spawn the real `rtz` binary.

### Changed

- **Coverage went from ~47% to ~77%**, and that number is now honest. The denominator used to
  count build-time-only code (the build script, network-download functions, the OS signal
  handler) that can't reasonably be unit tested; those are now excluded via a nightly-gated
  `#[cfg_attr(coverage_nightly, coverage(off))]` — so the exclusions apply under `cargo make
  codecov` (which runs on nightly) while the crate itself still builds and publishes on stable —
  and the dead `tarpaulin_include` markers are gone.
- **All dependencies bumped to latest** with clean version specs (`geo` 0.33, `geojson` 1,
  `schemars` 1, `rand` 0.10, `axum` 0.8, and the rest). This also resolved a `geo` 0.28/0.29 skew
  between `rtz` and `rtz-core` that had been sitting there unnoticed. **`bincode` stays pinned at
  `"2"`** on purpose: bumping to 3 breaks the unsafe zero-copy codec, so that one's an intentional
  exception to "always latest."
- **OSM admin data source is now `RTZ_OSM_ADMIN_DIRS`**, a semicolon-separated list of GeoJSON
  directories, instead of a hardcoded Windows path. `cargo xtask regen` sets it for you from
  whatever `extract-admin` produces. OSM-tz source pin also moved from `2024a` to `2026c`.
- **Docker build modernized**: `cargo-chef` planner/cook layers so dependency compilation is
  cached separately from source changes, a `cargo-chef` rust base that installs the repo's pinned
  toolchain, a slim `debian:stable-slim` runtime, and a proper `.dockerignore` so the build context
  isn't dragging `target/` and friends along.
- Removed the unused `WebResultMapper` trait and its dead helper functions from
  `rtz/src/web/types.rs`. Nobody was calling them; no reason to keep pretending otherwise.

### Fixed

- **The CLI rejected negative longitudes.** `rtz ned tz -87.62,41.88` failed because clap parsed
  the leading `-` as a flag instead of the start of a coordinate. Fixed with
  `allow_hyphen_values` on the lng/lat args in `rtz/src/bin.rs`.
