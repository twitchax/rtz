//! The [OpenStreetMap](https://www.openstreetmap.org/) timezone lookup module.

use std::{collections::HashMap, sync::OnceLock};

use geo::{Contains, Coord};
use rtz_core::{
    base::types::Float,
    geo::tz::{
        osm::OsmTimezone,
        shared::{i16_vec_to_tomezoneids, ConcreteVec, RoundLngLat, TimezoneIds},
    },
};

use super::shared::{HasCachedData, MapIntoTimezones};

/// Get the cache-driven timezone for a given longitude (x) and latitude (y).
///
/// The OSM database does allow overlap for disputed areas, so there can be multiple results.
pub fn get_timezones(xf: Float, yf: Float) -> Vec<&'static OsmTimezone> {
    let x = xf.floor() as i16;
    let y = yf.floor() as i16;

    let Some(timezones) = get_from_cache(x, y) else {
        return Vec::new();
    };

    // [ARoney] Optimization: If there is only one timezone, we can skip the more expensive
    // intersection check.  Edges are weird, so we still need to check if the point is in the
    // polygon at thg edges of the polar space.
    if timezones.len() == 1 && xf > -179. && xf < 179. && yf > -89. && yf < 89. {
        return timezones;
    }

    timezones.into_iter().filter(|&tz| tz.geometry.contains(&Coord { x: xf, y: yf })).collect()
}

/// Get the exact timezone for a given longitude (x) and latitude (y).
#[allow(dead_code)]
fn get_timezones_via_full_lookup(xf: Float, yf: Float) -> Vec<&'static OsmTimezone> {
    OsmTimezone::get_timezones().into_iter().filter(|&tz| tz.geometry.contains(&Coord { x: xf, y: yf })).collect()
}

/// Get value from the cache.
fn get_from_cache(x: i16, y: i16) -> Option<Vec<&'static OsmTimezone>> {
    let cache = OsmTimezone::get_cache();

    cache.get(&(x, y)).map_timezones()
}

// Trait impls.

impl HasCachedData for OsmTimezone {
    fn get_timezones() -> &'static ConcreteVec<OsmTimezone> {
        static TIMEZONES: OnceLock<ConcreteVec<OsmTimezone>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            TIMEZONES.get_or_init(|| {
                let (timezones, _len): (ConcreteVec<OsmTimezone>, usize) = bincode::serde::decode_from_slice(TZ_BINCODE, bincode::config::standard()).expect("Failed to decode timezones from binary: likely caused by precision difference between generated assets and current build.  Please rebuild the assets with `feature = \"force-rebuild\"` enabled.");

                timezones
            })
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::tz::{
                osm::GEOJSON_ADDRESS,
                shared::{get_geojson_features_from_string, get_timezones_from_features},
            };
            use std::io::Read;
            use zip::ZipArchive;

            TIMEZONES.get_or_init(|| {
                let response = reqwest::blocking::get(GEOJSON_ADDRESS).unwrap();
                let geojson_zip = response.bytes().unwrap();
                let mut zip = ZipArchive::new(std::io::Cursor::new(geojson_zip)).unwrap();
                let mut geojson_input = String::new();
                zip.by_index(0).unwrap().read_to_string(&mut geojson_input).unwrap();

                let features = get_geojson_features_from_string(&geojson_input);

                get_timezones_from_features(features)
            })
        }
    }

    fn get_cache() -> &'static HashMap<RoundLngLat, TimezoneIds> {
        static CACHE: OnceLock<HashMap<RoundLngLat, TimezoneIds>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            use rtz_core::geo::tz::shared::RoundInt;

            CACHE.get_or_init(|| {
                let (cache, _len): (HashMap<RoundLngLat, Vec<RoundInt>>, usize) = bincode::serde::decode_from_slice(CACHE_BINCODE, bincode::config::standard()).unwrap();

                cache
                    .into_iter()
                    .map(|(key, value)| {
                        let value = i16_vec_to_tomezoneids(value);

                        (key, value)
                    })
                    .collect::<HashMap<_, _>>()
            })
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::tz::shared::get_cache_from_timezones;

            CACHE.get_or_init(|| {
                let cache = get_cache_from_timezones(OsmTimezone::get_timezones());

                cache
                    .into_iter()
                    .map(|(key, value)| {
                        let value = i16_vec_to_tomezoneids(value);

                        (key, value)
                    })
                    .collect::<HashMap<_, _>>()
            })
        }
    }
}

// Statics.

#[cfg(all(host_family_unix, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("../../../../assets/osm_time_zones.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\osm_time_zones.bincode");

#[cfg(all(host_family_unix, feature = "self-contained"))]
static CACHE_BINCODE: &[u8] = include_bytes!("../../../../assets/osm_time_zone_cache.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static CACHE_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\osm_time_zone_cache.bincode");

// Tests.

#[cfg(test)]
mod tests {

    use super::super::shared::MapIntoTimezones;
    use super::*;
    use pretty_assertions::assert_eq;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    #[test]
    fn can_get_timezones() {
        let timezones = OsmTimezone::get_timezones();
        assert_eq!(timezones.len(), 444);
    }

    #[test]
    fn can_get_cache() {
        let cache = OsmTimezone::get_cache();
        assert_eq!(cache.len(), 64_800);

        // let a = cache.into_iter().max_by(|a, b| a.1.iter().filter(|x| **x != -1).collect::<Vec<_>>().len().cmp(&b.1.iter().filter(|x| **x != -1).collect::<Vec<_>>().len())).map(|(key, value)| {
        //     assert_eq!(format!("({}, {}): {}", key.0, key.1, value.iter().filter(|x| **x != -1).collect::<Vec<_>>().len()), "");
        // });
    }

    #[test]
    fn can_get_from_cache() {
        let cache = get_from_cache(-121, 46).unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn can_perform_exact_lookup() {
        assert_eq!(get_timezones_via_full_lookup(-177.0, -15.0).len(), 1);
        assert_eq!(get_timezones_via_full_lookup(-121.0, 46.0)[0].identifier, "America/Los_Angeles");

        assert_eq!(get_timezones_via_full_lookup(179.9968, -67.0959).len(), 1);
    }

    #[test]
    fn can_access_cache() {
        let cache = OsmTimezone::get_cache();

        let tzs = cache.get(&(-177, -15)).map_timezones().unwrap() as Vec<&OsmTimezone>;
        assert_eq!(tzs.len(), 1);

        let tzs = cache.get(&(-121, 46)).map_timezones().unwrap() as Vec<&OsmTimezone>;
        assert_eq!(tzs.len(), 1);

        let tz = cache.get(&(-121, 46)).map_timezones().unwrap()[0] as &OsmTimezone;
        assert_eq!(tz.identifier, "America/Los_Angeles");

        let tzs = cache.get(&(-87, 38)).map_timezones().unwrap() as Vec<&OsmTimezone>;
        assert_eq!(tzs.len(), 7);
    }

    #[test]
    fn can_verify_cache_assisted_accuracy() {
        (0..100).into_par_iter().for_each(|_| {
            let x = rand::random::<Float>() * 360.0 - 180.0;
            let y = rand::random::<Float>() * 180.0 - 90.0;
            let full = get_timezones_via_full_lookup(x, y);
            let cache_assisted = get_timezones(x, y);

            assert_eq!(
                full.into_iter().map(|t| t.id).collect::<Vec<_>>(),
                cache_assisted.into_iter().map(|t| t.id).collect::<Vec<_>>(),
                "({}, {})",
                x,
                y
            );
        });
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use rtz_core::base::types::Float;
    use test::{black_box, Bencher};

    use super::*;

    #[bench]
    #[ignore]
    fn bench_full_lookup_sweep(b: &mut Bencher) {
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezones_via_full_lookup(x as Float, y as Float));
                }
            }
        });
    }

    #[bench]
    fn bench_cache_assisted_sweep(b: &mut Bencher) {
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezones(x as Float, y as Float));
                }
            }
        });
    }

    #[bench]
    fn bench_worst_case_full_lookup_single(b: &mut Bencher) {
        let x = -86.5;
        let y = 38.5;

        b.iter(|| {
            black_box(get_timezones_via_full_lookup(x as Float, y as Float));
        });
    }

    #[bench]
    fn bench_worst_case_cache_assisted_single(b: &mut Bencher) {
        let x = -86.5;
        let y = 38.5;

        b.iter(|| {
            black_box(get_timezones(x, y));
        });
    }
}
