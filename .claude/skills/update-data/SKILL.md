---
name: update-data
description: Refresh rtz's embedded datasets — download the OSM planet PBF, extract admin boundaries, regenerate all bincodes from the latest sources (NED master, OSM-tz 2026c), verify they decode, and commit. Triggers on "update data", "refresh datasets", "regenerate bincodes", "update the rtz data".
---

# Updating rtz's embedded datasets

The bincodes in `rtz/assets/*.bincode` are generated, not hand-maintained. They were last
regenerated 2024.08.08. This skill drives `cargo xtask` to pull fresh source data, regenerate all
six bincodes, verify they decode, and (with your confirmation) commit the result.

Read this whole file before running anything — the pipeline downloads ~80GB and can run for hours.

## Prerequisites

- **The pinned Rust toolchain** — stable, pinned in `rust-toolchain.toml`; `cargo xtask` and the
  regen build both use it. `rustup show` should report the pinned toolchain as active in this repo.
  (Coverage is the only thing that needs nightly; the data regen does not.)
- **`curl`** on `PATH` — used for the resumable planet-PBF download.
- **~150GB of free disk** — the planet PBF is ~80GB, plus scratch space for `osm_extract_polygon`
  to write per-level admin GeoJSON.
- **Time and bandwidth** — the download alone is 80GB; extraction over the full planet is heavy on
  CPU and I/O. Budget for this to run unattended for a long while. Prefer a wired connection and a
  machine you can leave alone.

Do not kick this off on a whim — confirm with the operator that they actually want a multi-hour,
80GB job before starting.

## The pipeline

`cargo xtask` is a workspace member invoked via the `.cargo/config.toml` alias, so `cargo xtask
<cmd>` works from anywhere in the repo. It has five subcommands: `download-pbf`, `extract-admin`,
`regen`, `verify`, and `update` (which chains the first four).

### Easiest path: `cargo xtask update`

```bash
cargo xtask update
```

This downloads the planet PBF (resumable — safe to re-run if it drops), extracts admin boundaries,
regenerates all bincodes, and verifies them. **The first run will stop partway** — see the
two-phase handshake below for why, and just re-run the command it gives you.

### Step-by-step (or to resume after the handshake stop)

1. **Download the planet PBF** (skip if you already have one):

   ```bash
   cargo xtask download-pbf
   ```

   Lands in the gitignored `.rtz-data/` scratch dir by default. Resumable — `curl -C -` picks up
   where it left off if interrupted. Re-running the same command is always safe.

2. **Extract admin boundaries:**

   ```bash
   cargo xtask extract-admin --pbf .rtz-data/planet-latest.osm.pbf
   ```

   Installs `osm_extract_polygon` automatically if it's not on `PATH` (via `cargo install --git`).
   Runs it with `--min 2 --max 8`, writing one subdirectory per admin level under
   `.rtz-data/admin_data/` (the default `--out`).
   When it finishes, it prints the exact subdirectories it produced — copy that list for the next
   step.

3. **Regenerate the bincodes**, passing the admin dirs from step 2:

   ```bash
   cargo xtask regen --admin-dirs ".rtz-data/admin_data/admin2;...;.rtz-data/admin_data/admin8"
   ```

   Rebuilds with `full` + `force-rebuild`, which regenerates all six `rtz/assets/*.bincode` files
   from scratch — downloading NED `master` and OSM-tz `2026c` fresh in the process (the OSM admin
   data comes from the dirs you just extracted, not a download).

4. **Verify:**

   ```bash
   cargo xtask verify
   ```

   Runs `cargo nextest run --features web`, which decode-checks every regenerated bincode as part
   of the normal test suite. Green means the new bincodes are structurally sound; it does not mean
   the *data* is better — that's a judgment call for the operator, not this pipeline.

## The two-phase `--admin-dirs` handshake

`extract-admin` decides the per-level admin subdirectory names (`admin_data/admin2`,
`admin_data/admin3`, etc.) — they don't exist, and can't be predicted, until it actually runs. So
`regen` (and therefore `update`) needs `--admin-dirs` supplied *after* extraction completes.

`cargo xtask update` handles this by running `download-pbf` and `extract-admin` first. If you
didn't pass `--admin-dirs` up front, it stops there and prints the exact flag to re-run with:

```
extract-admin finished, but `--admin-dirs` was not provided, so `regen` cannot run.
Re-run `cargo xtask update` (or `cargo xtask regen`) with:
  --admin-dirs ".rtz-data/admin_data/admin2;...;.rtz-data/admin_data/admin8"
```

Copy that line verbatim into your next `cargo xtask update --pbf .rtz-data/planet-latest.osm.pbf
--admin-dirs "..."` (or straight into `cargo xtask regen`) — don't guess at the directory names
yourself.

## `RTZ_OSM_ADMIN_DIRS`

The OSM admin data source is the environment variable `RTZ_OSM_ADMIN_DIRS` — a semicolon-separated
list of GeoJSON directories — not a hardcoded path. `cargo xtask regen` sets it for you from
`--admin-dirs` before invoking the build. You should never need to set it by hand unless you're
calling `cargo build --features full --features force-rebuild` directly instead of going through
`xtask`.

## After `verify` is green

Do not commit automatically. Once `cargo xtask verify` passes:

1. Show the operator the asset diff:

   ```bash
   git diff --stat rtz/assets/
   ```

2. Wait for their explicit confirmation that the new data looks right (sane file-size deltas, no
   unexpected zero-byte files, etc.).

3. Only then stage and commit:

   ```bash
   git add rtz/assets
   git commit -m "data: regenerate bincodes from latest sources"
   ```

4. Reclaim disk when you're done — the gitignored `.rtz-data/` scratch dir holds the ~80GB PBF and
   the extracted admin GeoJSON:

   ```bash
   cargo xtask clean
   ```

   Cleanup is opt-in — `xtask` never deletes it automatically, so you can re-run the pipeline
   against the already-downloaded PBF without re-fetching 80GB.

Never commit on the operator's behalf without that confirmation step — a bad regen (truncated
download, a broken extraction) silently baked into the committed assets is worse than no update at
all.
