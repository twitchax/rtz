//! The build script crate for `rtz`.

#![cfg(not(target_family = "wasm"))]
#![warn(rustdoc::broken_intra_doc_links, rust_2018_idioms, clippy::all, missing_docs)]
#![allow(incomplete_features)]

#[cfg(feature = "self-contained")]
use std::path::PathBuf;

/// Main entry point for build script.
pub fn main() {
    #[cfg(feature = "self-contained")]
    generate_self_contained_bincodes();
}

/// Returns the assets directory of the crate currently being built.
///
/// Resolved via the `CARGO_MANIFEST_DIR` cargo sets for the build script at
/// runtime, so it lands on `rtz/assets` both in the workspace and in a
/// registry checkout (where the packaged crate ships the NED bincodes and
/// generated ones are written alongside them).
#[cfg(feature = "self-contained")]
fn assets_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo for build scripts")).join("assets")
}

#[cfg(feature = "self-contained")]
fn generate_self_contained_bincodes() {
    #[cfg(feature = "tz-ned")]
    generate_ned_tz_bincodes();
    #[cfg(feature = "tz-osm")]
    generate_osm_tz_bincodes();
    #[cfg(feature = "admin-osm")]
    generate_osm_admin_bincodes();
}

#[cfg(all(feature = "tz-ned", feature = "self-contained"))]
fn generate_ned_tz_bincodes() {
    use rtz_core::geo::{
        shared::generate_bincodes,
        tz::ned::{get_geojson_features_from_source, NedTimezone, LOOKUP_BINCODE_DESTINATION_NAME, TIMEZONE_BINCODE_DESTINATION_NAME},
    };

    let assets = assets_dir();
    let timezone_bincode_destination = assets.join(TIMEZONE_BINCODE_DESTINATION_NAME);
    let lookup_bincode_destination = assets.join(LOOKUP_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if timezone_bincode_destination.exists() && lookup_bincode_destination.exists() {
        return;
    }

    std::fs::create_dir_all(&assets).unwrap();

    let features = get_geojson_features_from_source();
    generate_bincodes::<NedTimezone>(features, timezone_bincode_destination, lookup_bincode_destination);
}

#[cfg(all(feature = "tz-osm", feature = "self-contained"))]
fn generate_osm_tz_bincodes() {
    use rtz_core::geo::{
        shared::generate_bincodes,
        tz::osm::{get_geojson_features_from_source, OsmTimezone, LOOKUP_BINCODE_DESTINATION_NAME, TIMEZONE_BINCODE_DESTINATION_NAME},
    };

    let assets = assets_dir();
    let timezone_bincode_destination = assets.join(TIMEZONE_BINCODE_DESTINATION_NAME);
    let lookup_bincode_destination = assets.join(LOOKUP_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if timezone_bincode_destination.exists() && lookup_bincode_destination.exists() {
        return;
    }

    std::fs::create_dir_all(&assets).unwrap();

    let features = get_geojson_features_from_source();
    generate_bincodes::<OsmTimezone>(features, timezone_bincode_destination, lookup_bincode_destination);
}

#[cfg(all(feature = "admin-osm", feature = "self-contained"))]
fn generate_osm_admin_bincodes() {
    use rtz_core::geo::{
        admin::osm::{get_geojson_features_from_source, OsmAdmin, ADMIN_BINCODE_DESTINATION_NAME, LOOKUP_BINCODE_DESTINATION_NAME},
        shared::generate_bincodes,
    };

    let assets = assets_dir();
    let admin_bincode_destination = assets.join(ADMIN_BINCODE_DESTINATION_NAME);
    let lookup_bincode_destination = assets.join(LOOKUP_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if admin_bincode_destination.exists() && lookup_bincode_destination.exists() {
        return;
    }

    std::fs::create_dir_all(&assets).unwrap();

    let features = get_geojson_features_from_source();
    generate_bincodes::<OsmAdmin>(features, admin_bincode_destination, lookup_bincode_destination);
}
