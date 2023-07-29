//! The build script crate for `rtz`.

#![cfg(not(target_family = "wasm"))]
#![warn(rustdoc::broken_intra_doc_links, rust_2018_idioms, clippy::all, missing_docs)]
#![allow(incomplete_features)]

/// Main entry point for build script.

pub fn main() {
    #[cfg(feature = "self-contained")]
    generate_self_contained_bincodes();
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
    use std::path::Path;

    use rtz_core::geo::{
        shared::generate_bincodes,
        tz::ned::{get_geojson_features_from_source, NedTimezone, LOOKUP_BINCODE_DESTINATION_NAME, TIMEZONE_BINCODE_DESTINATION_NAME},
    };

    let timezone_bincode_destination = &format!("../assets/{}", TIMEZONE_BINCODE_DESTINATION_NAME);
    let lookup_bincode_destination = &format!("../assets/{}", LOOKUP_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if Path::new(timezone_bincode_destination).exists() && Path::new(lookup_bincode_destination).exists() {
        return;
    }

    std::fs::create_dir_all("../assets").unwrap();

    let features = get_geojson_features_from_source();
    generate_bincodes::<NedTimezone>(features, timezone_bincode_destination, lookup_bincode_destination);
}

#[cfg(all(feature = "tz-osm", feature = "self-contained"))]
fn generate_osm_tz_bincodes() {
    use std::path::Path;

    use rtz_core::geo::{
        shared::generate_bincodes,
        tz::osm::{get_geojson_features_from_source, OsmTimezone, LOOKUP_BINCODE_DESTINATION_NAME, TIMEZONE_BINCODE_DESTINATION_NAME},
    };

    let timezone_bincode_destination = &format!("../assets/{}", TIMEZONE_BINCODE_DESTINATION_NAME);
    let lookup_bincode_destination = &format!("../assets/{}", LOOKUP_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if Path::new(timezone_bincode_destination).exists() && Path::new(lookup_bincode_destination).exists() {
        return;
    }

    std::fs::create_dir_all("../assets").unwrap();

    let features = get_geojson_features_from_source();
    generate_bincodes::<OsmTimezone>(features, timezone_bincode_destination, lookup_bincode_destination);
}

#[cfg(all(feature = "admin-osm", feature = "self-contained"))]
fn generate_osm_admin_bincodes() {
    use std::path::Path;

    use rtz_core::geo::{
        admin::osm::{get_geojson_features_from_source, OsmAdmin, ADMIN_BINCODE_DESTINATION_NAME, LOOKUP_BINCODE_DESTINATION_NAME},
        shared::generate_bincodes,
    };

    let admin_bincode_destination = &format!("../assets/{}", ADMIN_BINCODE_DESTINATION_NAME);
    let lookup_bincode_destination = &format!("../assets/{}", LOOKUP_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if Path::new(admin_bincode_destination).exists() && Path::new(lookup_bincode_destination).exists() {
        return;
    }

    std::fs::create_dir_all("../assets").unwrap();

    let features = get_geojson_features_from_source();
    generate_bincodes::<OsmAdmin>(features, admin_bincode_destination, lookup_bincode_destination);
}
