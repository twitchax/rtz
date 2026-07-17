//! Exercises the GeoJSON → items → lookup-cache pipeline against a small committed
//! fixture, so the pure preprocessing path is covered without any network download.
#![cfg(feature = "tz-ned")]

use rtz_core::geo::{
    shared::{get_geojson_features_from_string, get_items_from_features, get_lookup_from_geometries},
    tz::ned::NedTimezone,
};

const FIXTURE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/ne_10m_time_zones.test.geojson"));

#[test]
fn builds_items_and_lookup_from_fixture() {
    let features = get_geojson_features_from_string(FIXTURE);
    let items = get_items_from_features::<NedTimezone>(features);

    // The fixture has exactly three timezone features, order-preserved.
    assert_eq!(items.len(), 3);

    let first = &items[0];
    assert_eq!(first.description.as_ref(), "Arctic Ocean");
    assert_eq!(first.offset.as_ref(), "UTC-10:00");
    assert_eq!(first.zone, -10.0);
    assert_eq!(first.raw_offset, -36_000); // round(-10 * 3600)

    // The lookup cache covers every 1x1 degree cell of the globe: 360 * 180.
    let cache = get_lookup_from_geometries(&items);
    assert_eq!(cache.len(), 64_800);

    // Every referenced id points at a real item.
    for ids in cache.values() {
        for &id in ids.iter() {
            assert!((id as usize) < items.len(), "id {id} out of range");
        }
    }
}
