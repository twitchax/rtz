# Changelog

All notable changes to this project are documented here. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.10.0] - 2026-07-23

The OSM admin endpoint now returns results in a meaningful order — broadest-first — and, unusually,
that made lookups substantially faster rather than slower. Minor bump because the ordering is a new
behavioral guarantee and `rtz-core`'s public `HasGeometry` trait gained a method.

### Changed

- **OSM admin results are now returned broadest-first** — ascending by `level`, so a point inside
  nested areas yields the containment hierarchy in order (country, state, county, city) instead of
  source-file order. This is a *storage* property, not a query-time sort: `OsmAdmin::reorder` sorts
  the items at build time (keyed on `level`, with `relation_id` breaking ties so regens are
  deterministic), and since the lookup cache is built by walking items in order, the ordering costs
  nothing at runtime. `HasGeometry::reorder` defaults to identity, so the timezone datasets are
  untouched.
- **Admin lookups got substantially faster** as a side effect — grouping same-level (and so
  similarly-sized) geometries together improves locality on the scan. Measured with the existing
  criterion benches: full-scan worst case −20.5%, assisted sweep −13.5%, random cities −43.0%.
  This was expected to be a small regression, not a win; the numbers are reproducible via
  `cargo bench --features web --bench benches -- admin_osm`.
- `cargo xtask resort-admins` migrates already-generated admin bincodes into the new order without
  the source planet extract — both artifacts derive from the items blob, so a re-sort is ~3 minutes
  rather than a multi-hour, ~80GB re-ingest. `id` values shift as a result; they are documented as
  build-unstable, and `relationId` remains the durable identifier.

## [0.9.0] - 2026-07-22

A WASM/NPM release. The JS bindings returned a JSON string rather than objects — so the usage
documented since the first release never worked — and nothing tested that boundary. Both are
fixed, the package ships real TypeScript types, and the JS ABI now has CI coverage. Minor bump
because the `wasm` feature's exported signatures changed.

> **NPM version note.** `rtzweb@0.8.0` was published from this code before the release was cut,
> so it is identical to `0.9.0`. `0.9.0` restores the crate/NPM version lockstep; prefer it.

### Fixed

- **The WASM bindings returned a JSON string, not JS objects.** `getTimezoneNed(-121, 46)` handed
  back a `JsValue::from_str(...)` of serialized JSON, so the `tz.identifier` access documented in
  the README since the first release evaluated to `undefined` — consumers had to `JSON.parse` the
  result first. A later change to `lookup` also shifted the payload from a single object to an
  array without a note, breaking `JSON.parse(tz).identifier` as well. The bindings now return real
  JS arrays of objects via `serde-wasm-bindgen`. **Breaking for NPM consumers of `0.7.0` and
  earlier**; the README documents the migration. Nothing had ever tested the JS ABI, which is why
  this survived several releases — `cargo make test-wasm` now covers it under Node.

### Changed

- **NPM package hygiene.** Dropped `wee_alloc` (unmaintained since 2022, with a known unfixed
  leak, and worth nothing here — the package is ~99% static dataset, and removing it made the
  `.wasm` *smaller*). Removed `js-sys` and `wasm-bindgen-futures`, which were declared in the
  `wasm` feature but referenced nowhere. Serialization failures now surface as thrown JS
  exceptions rather than panics, which in WASM trap and poison the module instance.
- **The NPM package ships real TypeScript types.** Every binding was typed `any`; the response
  structs now derive `Tsify` alongside the existing `ToSchema`, so `rtzlib.d.ts` carries real
  interfaces — including the Rust doc comments — generated from the same definitions the server
  uses. `getTimezoneNed` is now `(lng: number, lat: number) => NedTimezoneResponse1[]`.
- **`cargo make wasm` / `cargo make test-wasm`** replace the hand-run `wasm-pack` invocation and
  the manual `rtzweb` rename in `pkg/package.json` (which reverted on every build, so a forgetful
  release would publish under the wrong package name).
- **WASI distribution moved from wasmer to a WASI Preview 2 component.** Releases now attach
  `rtz-wasm32-wasip2.wasm` to the GitHub release, runnable with any component-capable runtime
  (`wasmtime run rtz-wasm32-wasip2.wasm ned tz 30,30`) — no registry, no manifest. The wasmer
  channel is deprecated and frozen at `0.8.0`: its runtime can't execute Preview 2 components,
  which pinned us to the legacy Preview 1 target. `wasmer.toml` removed.

## [0.8.0] - 2026-07-19

A tooling and coverage pass, a full dependency sweep, a new pipeline for refreshing the embedded
datasets — and the first refresh through it: all six bincodes regenerated from the latest upstream
sources. Minor bump because the `geo` 0.28 → 0.33 update is exposed through the public geometry
types (`Geometry<Float>`), and `OsmAdmin` gained a field.

### Added

- **`cargo xtask` data-update pipeline.** New `xtask` crate with `download-pbf`, `extract-admin`,
  `regen`, `verify`, `update`, and `clean` subcommands that chain them end to end: resumable
  planet-PBF download, admin-boundary extraction via `osm_extract_polygon`, a full bincode regen
  against the latest NED/OSM sources, and a decode-check via `cargo nextest`. Large artifacts (the
  ~80GB PBF, extracted GeoJSON) default into a gitignored `.rtz-data/` scratch dir, and `cargo
  xtask clean` reclaims it. Paired with a repo-local `update-data` project skill
  (`.claude/skills/update-data/SKILL.md`) that walks an operator through it.
- **Stable OSM `relationId` on the admin API.** `OsmAdmin` now carries the OSM `relation_id` from
  the source (e.g. Egypt = `1473947`), surfaced as `relationId` on `/osm/admin` responses. Unlike
  the existing `id` — a build-local index used to address the lookup cache — it's stable across
  builds and datasets, giving consumers a durable identifier. Additive; `id` is unchanged.
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
- **All six embedded bincodes regenerated from the latest sources.** NED `master`, OSM-tz `2026c`,
  and a fresh planet admin extraction (levels 2-8). `osm_time_zones` shrank ~7.5MB → ~2.6MB under
  geo-0.33 simplification (99.99% agreement with the finer epsilon); `osm_admins` grew with OSM
  coverage and the new `relation_id`. The OSM admin/tz tests were made regen-stable in the process
  (set membership, floors, and the stable `relationId`/`identifier` instead of exact counts,
  build-local ids, and result ordering — all of which legitimately shift on every refresh).
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
- **Zero-copy geometry could `dealloc` static memory.** The `self-contained` borrow-decode path
  rebuilds coordinate `Vec`s over the embedded `.rodata` bytes via `Vec::from_raw_parts`; letting
  `geo`'s `Vec::drop` run on those would free a pointer the allocator never handed out (UB). It was
  only sound because the geometries live in a process-lifetime static. `EncodableGeometry` now has a
  `Drop` (present only under the default borrow decode, gated off by `owned-decode`) that forgets the
  static-backed geometry, so the dealloc can't happen; `owned-decode` builds still drop normally.
- **The `cargo xtask` data pipeline didn't actually run end-to-end.** `regen` passed a relative
  `--admin-dirs` that the build script (CWD = `rtz/`) couldn't resolve, and `update` assumed
  per-level `adminN/` subdirectories that `osm_extract_polygon` doesn't produce (it writes one flat
  file per area). Both fixed; the skill docs corrected to match.
