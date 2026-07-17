# rtz Data-Update Pipeline Plan (Goal 3)

**Goal:** Automate refreshing the embedded datasets — download planet PBF, extract admin boundaries, regenerate all bincodes from the **latest** sources, verify they decode, and commit — via a repo-local skill orchestrating a `cargo xtask`.

**Decisions (locked):** scope = regen + verify + commit (publish/deploy stay manual); PBF downloaded in-pipeline (resumable); deliverable = skill + `cargo xtask`; admin source de-hardcoded to env `RTZ_OSM_ADMIN_DIRS`; grab latest data (NED `master`, OSM-tz `2026c`, `planet-latest.osm.pbf`); update README + add CHANGELOG.

**De-risk (DONE):** Regenerated NED on `geo 0.33` and decoded it — all 6 NED tests pass. The D2 `capacity_overflow` was a transient artifact of a concurrent `--all-features` build, not a codec/geo bug. Regeneration is sound.

**Global constraints:** nightly `nightly-2026-07-13`; gate `cargo make test` (42) + `cargo make clippy` (0); work on `main`, **unstaged, no commit**; **NEVER `--all-features` / `force-rebuild` in a normal build** (regenerates committed assets); scoped git ops only.

---

### T2 — De-hardcode admin source + latest dataset pins

**Files:** `rtz-core/src/geo/admin/osm.rs`, `rtz-core/src/geo/tz/osm.rs`.
- In `admin/osm.rs`: replace the hardcoded Windows `ADDRESS` static with a read of env `RTZ_OSM_ADMIN_DIRS` (semicolon-separated dirs) inside `get_geojson_features_from_source`, erroring clearly if unset (this fn only runs during a regen). Remove/replace the stale `pub static ADDRESS` (check it's referenced nowhere else first).
- In `tz/osm.rs`: bump the pinned release in `ADDRESS` from `2024a` → `2026c` (latest timezone-boundary-builder). NED stays `master` (already latest).
- Green-gate: `cargo make test` (42) + `cargo make clippy` (0). Normal builds use the committed bincodes (self-contained) and never call these fns, so the suite stays green.

### T3 — `xtask` crate: the pipeline

**Files:** new `xtask/` workspace member (`xtask/Cargo.toml`, `xtask/src/main.rs`); add to root `[workspace] members`.
- `cargo xtask update-data` with subcommands, orchestrating (shelling out where a battle-tested tool is right):
  - `download-pbf [--url] [--out]` — resumable download of `planet-latest.osm.pbf` via `curl -L -C -` (80 GB; log size/space warnings).
  - `extract-admin --pbf <path> --out <dir>` — ensure `osm_extract_polygon` is available (`cargo install --git https://github.com/AndGem/osm_extract_polygon` if missing), then run it `-g -o --min 2 --max 8 --path <dir> -f <pbf>` (Aaron's exact form) → `admin2…admin8/`.
  - `regen [--admin-dirs <dirs>]` — run `cargo build --features full --features force-rebuild` with `RTZ_OSM_ADMIN_DIRS` set (f32; never `--all-features`), regenerating all six bincodes.
  - `verify` — decode every regenerated bincode + sanity lookups, then `cargo nextest run --features web`; fail loudly on a decode crash.
  - `update` — download-pbf → extract-admin → regen → verify, end-to-end.
- **Testing reality:** the small paths (`regen` for NED/tz, `verify`) are exercised here as the de-risk was; the 80 GB `download-pbf`/`extract-admin`/admin-`regen` paths are validated by construction + a dry-run (`--help`, arg wiring, a tiny fake-input smoke where feasible) — a full run needs a real PBF on the operator's machine. Say this in the report.
- Green-gate: `cargo make test` + `cargo make clippy` still 42/0 (xtask is its own crate; keep it out of the default test feature set so it doesn't slow the suite).

### T4 — The skill

**Files:** `.claude/skills/update-data/SKILL.md` (repo-local).
- Frontmatter (name, description, triggers: "update data", "refresh datasets", "regenerate bincodes"). Body: prerequisites (disk ≥ ~150 GB, bandwidth, nightly), then drive `cargo xtask update-data` step-by-step, surface progress, run `cargo make test`, then `git add rtz/assets` + commit **on operator confirmation** (show the asset diff first). Document the env var and the "latest sources" it pulls.

### T5 — Docs + release notes

**Files:** `README.md`, new `CHANGELOG.md`.
- README "Data Updates": replace the manual hand-wave with the `cargo xtask update-data` flow + `RTZ_OSM_ADMIN_DIRS`, update the sources (NED master, OSM-tz **2026c**, planet-latest) and the "last updated" date.
- `CHANGELOG.md` (new, Keep-a-Changelog style) — release notes covering this whole session's body of work: nightly + cargo-make/nextest/llvm-cov migration; coverage 47→77%; the CLI negative-coordinate bug fix; dead-code removal; Docker modernization; dependency bumps (geo/geojson/… , bincode held at 2); and the data-update pipeline. Group under an `## [Unreleased]` heading.
