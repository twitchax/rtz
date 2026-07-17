# rtz Dependency Bumps Plan (Goal 2)

**Goal:** Bring every dependency to its latest version with clean version specs, matching the author's house convention, resolving the `geo` version skew, and keeping the suite green.

**Policy (locked):**
- Every dep → latest. Spec form: `"MAJOR"` for ≥1.0 crates (e.g. `serde = "1"`), `"0.MINOR"` for 0.x crates (e.g. `geo = "0.33"`). Applies to all three `Cargo.toml`s and dev/build-deps.
- **`bincode` stays `"2"`** — do NOT bump to 3 (breaks the unsafe zero-copy codec; deliberate exception).
- Internal path-dep `version = "…"` fields (rtz-core, rtz-build) are left as-is (they track the crates' own versions).

**Global constraints:**
- Nightly `nightly-2026-07-13`; canonical gate `cargo make test` (must stay **42 passed**) + `cargo make clippy` (exit 0).
- Work on `main`, **no branch, no commit, no staging** — everything unstaged for review.
- Migrations are compile-driven: bump spec → `cargo update -p <crate>` (or `cargo update`) → build → fix breakage → green.

**Order (breakage isolated per cluster, green-gate after each):**

---

### D1 — Clean specs + safe (non-major) bumps

Rewrite the version spec of every dependency to the clean convention, **keeping each at its current major/minor** (these are already at their latest major), across `rtz/Cargo.toml`, `rtz-core/Cargo.toml`, and `rtz/Cargo.toml` dev/build/target tables. Preserve every `features`, `optional`, and `default-features` setting — change only the `version` string.

Concrete targets (non-exhaustive; apply the rule to ALL deps except the D2–D4 list below):
`anyhow="1"`, `rayon="1"`, `chashmap="2"`, `serde="1"`, `serde_json="1"`, `include_bytes_aligned="0.1"`, `clap="4"`, `chrono-tz="0.10"`, `chrono="0.4"`, `tokio="1"`, `config="0.15"`, `log="0.4"`, `simple_logger="5"`, `axum="0.8"`, `hyper="1"`, `tower="0.5"`, `http="1"`, `http-body-util="0.1"`, `utoipa="5"`, `utoipa-swagger-ui="9"`, `utoipa-redoc="6"`, `utoipa-rapidoc="6"`, `axum-insights="0.6"`, `tracing="0.1"`, `wasm-bindgen="0.2"`, `wasm-bindgen-futures="0.4"`, `wee_alloc="0.4"`, `js-sys="0.3"`; dev-deps `pretty_assertions="1"`, `cities-json="0.6"`, `futures="0.3"`, `criterion="0.7"` (use the true latest major of criterion — check `cargo info criterion` from `/tmp`). **`bincode` unchanged (`"2"`).**

**Do NOT touch in D1** (handled later): `geo`, `geojson`, `schemars`, `rand`, `getrandom`, `tower-http`, `reqwest`, `zip`.

Then `cargo update`, `cargo make test` (42), `cargo make clippy` (0). If a "clean" spec resolves to a NEW major that breaks (shouldn't, since these are already at latest major), pin it back and flag it.

---

### D2 — `geo` → `"0.33"` (both crates: unify + bump)

Set `geo = "0.33"` in **both** `rtz/Cargo.toml` and `rtz-core/Cargo.toml` (this resolves the 0.28/0.29 skew and bumps to latest). `cargo update -p geo`. Fix the 22 call sites (mostly `rtz-core/src/geo/shared.rs`, plus `rtz/src/geo/shared.rs`, `rtz/src/geo/tz/ned.rs`): the migration surface is `use geo::{…, SimplifyVw}` + `.simplify_vw(&epsilon)`, `Contains`/`.contains()`, `Intersects`/`.intersects()`, `Coord{x,y}`, `Rect::new`, `Polygon`/`MultiPolygon`/`LineString`/`Geometry`. Consult geo 0.30–0.33 changelogs for renames (e.g. simplify trait/method). Green-gate. The `can_verify_lookup_assisted_accuracy` tests exercise real geometry — they must still pass.

---

### D3 — `geojson` → `"1"` (both crates)

Set `geojson = "1"` in both crates. `cargo update -p geojson`. Fix the ~56 sites in `rtz-core` (`geo/shared.rs`, `geo/tz/ned.rs`, `geo/tz/osm.rs`, `geo/admin/osm.rs`): `FeatureCollection`, `GeoJson`, `Feature`, `geojson::Geometry`, and the `geometry.value.clone().try_into()` → `geo::Geometry` conversions (geojson 1.0 reworked the geo-types conversion / may need the `geo-types` feature). Green-gate — the ingestion fixture test (`rtz-core/tests/ingestion.rs`) and the runtime lookups validate it.

---

### D4 — Remaining majors

Apply, each with `cargo update -p <crate>` + a build, fixing breakage:
- `schemars = "1"` **and** rename its feature `chrono` → `chrono04` in `rtz/Cargo.toml` (schemars is Cargo-only, no source changes).
- `rand = "0.10"` — fix `rand::random::<Float>()` in the 3 test files if the API moved (0.10 may keep `rand::random`; adjust to `rand::random()` / `rng().random()` as needed).
- `getrandom = "0.4"` — keep the `wasm_js` feature (verify it still exists in 0.4; adjust if renamed).
- `tower-http = "0.7"`, `reqwest = "0.13"`, `zip = "9"` (or true latest) — build-time / middleware; fix API breakage (`reqwest`/`zip` only used in `get_geojson_features_from_source`).

Split into sub-steps if breakage tangles. Green-gate at the end (42 + clippy 0).

---

### D5 — Final sweep

- `cargo update` (full), then `cargo make test` (42), `cargo make clippy` (0), `cargo build --features web --release` (release compiles).
- Confirm every external dep spec is clean (`"1"` / `"0.N"`); no `X.Y.Z` triples remain except intentional (`bincode="2"`, internal path deps).
- Re-run `cargo make codecov` to confirm coverage is unaffected (~77%).
- Report the final version delta table for Aaron.
