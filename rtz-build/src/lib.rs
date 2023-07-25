//! The build script crate for `rtz`.

#![cfg(not(target_family = "wasm"))]
#![warn(rustdoc::broken_intra_doc_links, rust_2018_idioms, clippy::all, missing_docs)]
#![allow(incomplete_features)]

/// Main entry point for build script.
#[cfg(feature = "self-contained")]
pub fn main() {
    generate_self_contained_bincodes();
}

#[cfg(feature = "self-contained")]
fn generate_self_contained_bincodes() {
    #[cfg(feature = "tz-ned")]
    generate_ned_bincodes();
    #[cfg(feature = "tz-osm")]
    generate_osm_bincodes();
}

#[cfg(all(feature = "tz-ned", feature = "self-contained"))]
fn generate_ned_bincodes() {
    use std::path::Path;

    use rtz_core::geo::tz::{ned::{TIMEZONE_BINCODE_DESTINATION_NAME, CACHE_BINCODE_DESTINATION_NAME, NedTimezone}, shared::{get_geojson_features_from_string, generate_bincodes}};

    let timezone_bincode_destination = &format!("../assets/{}", TIMEZONE_BINCODE_DESTINATION_NAME);
    let cache_bincode_destination = &format!("../assets/{}", CACHE_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if Path::new(timezone_bincode_destination).exists() && Path::new(cache_bincode_destination).exists() {
        return;
    }

    std::fs::create_dir_all("../assets").unwrap();

    let response = reqwest::blocking::get(rtz_core::geo::tz::ned::GEOJSON_ADDRESS).unwrap();
    let geojson_input = response.text().unwrap();

    let features = get_geojson_features_from_string(&geojson_input);
    generate_bincodes::<NedTimezone>(features, timezone_bincode_destination, cache_bincode_destination);
}

#[cfg(all(feature = "tz-osm", feature = "self-contained"))]
fn generate_osm_bincodes() {
    use std::{path::Path, io::Read};

    use rtz_core::geo::tz::{osm::{TIMEZONE_BINCODE_DESTINATION_NAME, CACHE_BINCODE_DESTINATION_NAME, OsmTimezone}, shared::{get_geojson_features_from_string, generate_bincodes}};
    use zip::ZipArchive;

    let timezone_bincode_destination = &format!("../assets/{}", TIMEZONE_BINCODE_DESTINATION_NAME);
    let cache_bincode_destination = &format!("../assets/{}", CACHE_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if Path::new(timezone_bincode_destination).exists() && Path::new(cache_bincode_destination).exists() {
        return;
    }

    std::fs::create_dir_all("../assets").unwrap();

    let response = reqwest::blocking::get(rtz_core::geo::tz::osm::GEOJSON_ADDRESS).unwrap();
    let geojson_zip = response.bytes().unwrap();
    let mut zip = ZipArchive::new(std::io::Cursor::new(geojson_zip)).unwrap();
    let mut geojson_input = String::new();
    zip.by_index(0).unwrap().read_to_string(&mut geojson_input).unwrap();

    //let geojson_input = response.text().unwrap();

    let features = get_geojson_features_from_string(&geojson_input);
    generate_bincodes::<OsmTimezone>(features, timezone_bincode_destination, cache_bincode_destination);
}
