#![cfg(not(target_family = "wasm"))]

pub fn main() {
    #[cfg(feature = "self-contained")]
    generate_self_contained_bincodes();
}

#[cfg(feature = "self-contained")]
fn generate_self_contained_bincodes() {
    #[cfg(feature = "tz-ned")]
    generate_ned_bincodes();
}

#[cfg(all(feature = "tz-ned", feature = "self-contained"))]
fn generate_ned_bincodes() {
    use std::path::Path;

    use rtz_core::geo::tz::ned::{TIMEZONE_BINCODE_DESTINATION_NAME, CACHE_BINCODE_DESTINATION_NAME};

    let timezone_bincode_destination = &format!("../assets/{}", TIMEZONE_BINCODE_DESTINATION_NAME);
    let cache_bincode_destination = &format!("../assets/{}", CACHE_BINCODE_DESTINATION_NAME);

    #[cfg(not(feature = "force-rebuild"))]
    if Path::new(timezone_bincode_destination).exists() && Path::new(cache_bincode_destination).exists() {
        return;
    }

    std::fs::create_dir_all("../assets").unwrap();

    let response = reqwest::blocking::get(rtz_core::geo::tz::ned::GEOJSON_ADDRESS).unwrap();
    let geojson_input = response.text().unwrap();

    let features = rtz_core::geo::tz::ned::get_geojson_features_from_string(&geojson_input);
    rtz_core::geo::tz::ned::generate_bincodes(features, timezone_bincode_destination, cache_bincode_destination);
}
