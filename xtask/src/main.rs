//! `cargo xtask` — orchestrates the `rtz` data-update pipeline.
//!
//! This crate deliberately does very little work itself: it shells out to
//! battle-tested tools (`curl`, `osm_extract_polygon`, `cargo`) and focuses on
//! sequencing them correctly, validating preconditions, and printing clear
//! progress / warning messages. See each subcommand's `--help` text for
//! details, and the top-level `update` subcommand for the full pipeline.
//!
//! The one exception is `resort-admins`, which calls into `rtz-core` directly
//! rather than shelling out — it rewrites the committed bincodes in place, and
//! the encode/decode logic that knows their layout lives there.

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

/// `cargo xtask` — orchestrates the `rtz` data-update pipeline.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Download the OSM planet PBF file (used as input to `extract-admin`).
    DownloadPbf {
        /// URL of the planet PBF to download.
        #[arg(long, default_value = "https://planet.openstreetmap.org/pbf/planet-latest.osm.pbf")]
        url: String,

        /// Output path for the downloaded PBF file (under the gitignored `.rtz-data/` scratch dir by default).
        #[arg(long, default_value = ".rtz-data/planet-latest.osm.pbf")]
        out: PathBuf,
    },

    /// Extract per-level administrative boundary GeoJSON from a planet PBF
    /// using `osm_extract_polygon` (installed automatically if missing).
    ExtractAdmin {
        /// Path to the source planet PBF file (e.g. from `download-pbf`).
        #[arg(long)]
        pbf: PathBuf,

        /// Output directory for the extracted admin GeoJSON.
        #[arg(long, default_value = ".rtz-data/admin_data")]
        out: PathBuf,
    },

    /// Regenerate all six data bincodes into `rtz/assets/`, downloading NED
    /// (master) and OSM-tz (2026c) fresh in the process.
    Regen {
        /// Semicolon-separated list of admin GeoJSON directories produced by
        /// `extract-admin` (passed through as `RTZ_OSM_ADMIN_DIRS`).
        #[arg(long)]
        admin_dirs: String,
    },

    /// Recompile with the freshly-embedded bincodes and run the test suite
    /// (via `cargo nextest run --features web`) to catch a bad regen.
    Verify,

    /// Run the full pipeline: download-pbf (if needed) -> extract-admin ->
    /// regen -> verify.
    Update {
        /// Path to an existing planet PBF file. If omitted, one is downloaded
        /// to the default `download-pbf` output path first.
        #[arg(long)]
        pbf: Option<PathBuf>,

        /// Output directory for the extracted admin GeoJSON (passed to
        /// `extract-admin`).
        #[arg(long, default_value = ".rtz-data/admin_data")]
        admin_out: PathBuf,

        /// Semicolon-separated list of admin GeoJSON directories to use for
        /// `regen`. If omitted, `extract-admin` runs first and this errors
        /// out with instructions to re-run using the dirs it produced.
        #[arg(long)]
        admin_dirs: Option<String>,
    },

    /// Re-sort the committed OSM admin bincodes into the canonical order
    /// (`OsmAdmin::reorder`) and rebuild their lookup cache, without needing
    /// the source planet extract.
    ///
    /// A one-time migration for assets generated before the ordering existed;
    /// `regen` applies the same order to anything it produces.
    ResortAdmins,

    /// Remove the `.rtz-data/` scratch directory (downloaded PBF + extracted
    /// admin GeoJSON) to reclaim disk space.
    Clean,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let repo_root = repo_root();

    match args.command {
        Cmd::DownloadPbf { url, out } => download_pbf(&repo_root, &url, &out),
        Cmd::ExtractAdmin { pbf, out } => extract_admin(&repo_root, &pbf, &out).map(|_| ()),
        Cmd::Regen { admin_dirs } => regen(&repo_root, &admin_dirs),
        Cmd::Verify => verify(&repo_root),
        Cmd::Update { pbf, admin_out, admin_dirs } => update(&repo_root, pbf, &admin_out, admin_dirs),
        Cmd::ResortAdmins => resort_admins(&repo_root),
        Cmd::Clean => clean(&repo_root),
    }
}

/// Resolves the repository root from `CARGO_MANIFEST_DIR` (i.e. `xtask/..`),
/// so subcommands behave the same regardless of the operator's current
/// directory when they run `cargo xtask ...`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/Cargo.toml always has a parent directory")
        .to_path_buf()
}

/// Runs `command` in `cwd`, with optional extra environment variables,
/// streaming its stdout/stderr straight through, and returns an error with
/// `context` if the tool is missing or exits non-zero.
fn run(cwd: &Path, mut command: Command, context: &str) -> Result<()> {
    let status = command
        .current_dir(cwd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to launch: {context}"))?;

    if !status.success() {
        bail!("{context} (exit status: {status})");
    }

    Ok(())
}

/// Returns `true` if `program` is resolvable on `PATH`.
fn on_path(program: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file() || candidate.with_extension("exe").is_file()
    })
}

// Subcommand implementations.

/// `download-pbf`: resumably downloads the planet PBF via `curl`.
fn download_pbf(repo_root: &Path, url: &str, out: &Path) -> Result<()> {
    if !on_path("curl") {
        bail!("`curl` was not found on PATH — install it and re-run `cargo xtask download-pbf`.");
    }

    println!("warning: the OSM planet PBF is roughly 80GB. Ensure you have ~150GB of free disk space");
    println!("         (room for the download plus scratch space for `extract-admin`) before continuing.");
    println!("downloading planet PBF from {url} to {}", out.display());
    println!("(the download is resumable — re-running this command will pick up where it left off)");

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(repo_root.join(parent)).with_context(|| format!("failed to create output directory {}", parent.display()))?;
        }
    }

    let mut command = Command::new("curl");
    command.args(["-L", "-C", "-", "-o"]).arg(out).arg(url);

    run(repo_root, command, "curl failed to download the planet PBF")?;

    println!("downloaded planet PBF to {}", out.display());

    Ok(())
}

/// `extract-admin`: ensures `osm_extract_polygon` is installed, then runs it
/// against `pbf`, producing one GeoJSON file per admin area (levels 2-8) flat
/// under `out`. Returns `out` on success — that directory is the `--admin-dirs`
/// value for `regen`.
fn extract_admin(repo_root: &Path, pbf: &Path, out: &Path) -> Result<PathBuf> {
    if !pbf.exists() {
        bail!("PBF file not found at {} — run `cargo xtask download-pbf` first, or pass `--pbf` to an existing file.", pbf.display());
    }

    if !on_path("osm_extract_polygon") {
        println!("`osm_extract_polygon` was not found on PATH — installing it via `cargo install` (this may take a while)...");

        let mut command = Command::new("cargo");
        command.args(["install", "--git", "https://github.com/AndGem/osm_extract_polygon"]);

        run(repo_root, command, "failed to install osm_extract_polygon via `cargo install --git`")?;
    }

    println!("extracting admin boundaries (levels 2-8) from {} into {}...", pbf.display(), out.display());

    std::fs::create_dir_all(repo_root.join(out)).with_context(|| format!("failed to create output directory {}", out.display()))?;

    let mut command = Command::new("osm_extract_polygon");
    command
        .args(["-g", "-o", "--min", "2", "--max", "8", "--path"])
        .arg(out)
        .arg("-f")
        .arg(pbf);

    run(repo_root, command, "osm_extract_polygon failed to extract admin boundaries")?;

    println!("extracted admin boundaries to {}", out.display());
    println!("note: all admin areas are written as individual GeoJSON files flat under that");
    println!("      directory — pass it directly as `--admin-dirs` to `cargo xtask regen`.");

    Ok(out.to_path_buf())
}

/// `regen`: rebuilds `rtz` with `full` + `force-rebuild`, regenerating all
/// six bincodes into `rtz/assets/`.
fn regen(repo_root: &Path, admin_dirs: &str) -> Result<()> {
    // The build script that reads `RTZ_OSM_ADMIN_DIRS` runs with its CWD set to the `rtz` crate
    // directory (cargo always runs build scripts there), not the workspace root or wherever the
    // operator invoked xtask. A relative dir like `.rtz-data/admin_data` would resolve against
    // `rtz/` and not be found, so absolutize each dir against the repo root before handing it off.
    let admin_dirs = admin_dirs
        .split(';')
        .map(|dir| {
            let dir = Path::new(dir.trim());
            let abs = if dir.is_absolute() { dir.to_path_buf() } else { repo_root.join(dir) };
            abs.to_string_lossy().into_owned()
        })
        .collect::<Vec<_>>()
        .join(";");

    println!("regenerating all data bincodes into rtz/assets/ — this downloads NED (master) and OSM-tz (2026c) fresh");
    println!("and re-encodes everything from scratch. RTZ_OSM_ADMIN_DIRS={admin_dirs}");

    let mut command = Command::new("cargo");
    command
        .args(["build", "--features", "full", "--features", "force-rebuild"])
        .env("RTZ_OSM_ADMIN_DIRS", &admin_dirs);

    run(repo_root, command, "cargo build failed while regenerating the data bincodes")?;

    println!("regenerated bincodes in rtz/assets/");

    Ok(())
}

/// `verify`: recompiles with the freshly-embedded bincodes and runs the full
/// test suite, which decodes them via the geo tests.
fn verify(repo_root: &Path) -> Result<()> {
    println!("verifying the freshly-regenerated bincodes via `cargo nextest run --features web`...");

    let mut command = Command::new("cargo");
    command.args(["nextest", "run", "--features", "web"]);

    match run(repo_root, command, "cargo nextest run failed") {
        Ok(()) => {
            println!("verify: PASSED — the regenerated bincodes decode correctly.");
            Ok(())
        }
        Err(err) => {
            println!("verify: FAILED — the regenerated bincodes did not pass the test suite.");
            Err(err)
        }
    }
}

/// `update`: chains `download-pbf` (if needed) -> `extract-admin` -> `regen`
/// -> `verify`.
///
/// `extract-admin` writes every admin area as its own GeoJSON file flat under
/// one output directory, so that directory is the admin source directly — no
/// per-level handshake. Pass `--admin-dirs` only to override it (e.g. to reuse
/// a prior extraction without re-running `extract-admin`).
fn update(repo_root: &Path, pbf: Option<PathBuf>, admin_out: &Path, admin_dirs: Option<String>) -> Result<()> {
    let pbf = match pbf {
        Some(pbf) => pbf,
        None => {
            let default_out = PathBuf::from(".rtz-data/planet-latest.osm.pbf");
            download_pbf(repo_root, "https://planet.openstreetmap.org/pbf/planet-latest.osm.pbf", &default_out)?;
            default_out
        }
    };

    let produced_dir = extract_admin(repo_root, &pbf, admin_out)?;

    // The admin source is the flat directory `extract-admin` produced, unless the operator
    // explicitly overrode it (e.g. to reuse an earlier extraction).
    let admin_dirs = admin_dirs.unwrap_or_else(|| produced_dir.display().to_string());

    regen(repo_root, &admin_dirs)?;
    verify(repo_root)?;

    println!("update: pipeline complete.");

    Ok(())
}

/// `clean`: removes the `.rtz-data/` scratch directory (downloaded PBF +
/// extracted admin GeoJSON) to reclaim disk space.
fn resort_admins(repo_root: &Path) -> Result<()> {
    let assets = repo_root.join("rtz").join("assets");
    let items = assets.join("osm_admins.bincode");
    let lookup = assets.join("osm_admin_lookup.bincode");

    for path in [&items, &lookup] {
        if !path.exists() {
            bail!("{} does not exist — run `cargo xtask regen` first.", path.display());
        }
    }

    println!("re-sorting {} and rebuilding {} ...", items.display(), lookup.display());
    rtz_core::geo::shared::resort_items_bincode::<rtz_core::geo::admin::osm::OsmAdmin>(&items, &lookup);
    println!("done. re-run `cargo xtask verify` to confirm the assets still decode.");

    Ok(())
}

fn clean(repo_root: &Path) -> Result<()> {
    let scratch = repo_root.join(".rtz-data");

    if !scratch.exists() {
        println!("nothing to clean — {} does not exist.", scratch.display());
        return Ok(());
    }

    println!("removing scratch directory {} ...", scratch.display());
    std::fs::remove_dir_all(&scratch).with_context(|| format!("failed to remove {}", scratch.display()))?;
    println!("cleaned up {}.", scratch.display());

    Ok(())
}
