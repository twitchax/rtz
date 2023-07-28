//! The [Natural Earth Data](https://github.com/nvkelso/natural-earth-vector) timezone lookup module.

use std::{collections::HashMap, sync::OnceLock};

use geo::{Contains, Coord};
use rtz_core::{
    base::types::Float,
    geo::{
        shared::{ConcreteVec, RoundLngLat, ToGeoJson},
        tz::{
            ned::NedTimezone,
            shared::{i16_vec_to_tomezoneids, TimezoneIds},
        },
    },
};

use super::shared::{HasLookupData, MapIntoTimezones};

/// Get the cache-driven timezone for a given longitude (x) and latitude (y).
pub fn get_timezone(xf: Float, yf: Float) -> Option<&'static NedTimezone> {
    let x = xf.floor() as i16;
    let y = yf.floor() as i16;

    let timezones = get_from_lookup(x, y)?;

    // [ARoney] Optimization: If there is only one timezone, we can skip the more expensive
    // intersection check.  Edges are weird, so we still need to check if the point is in the
    // polygon at thg edges of the polar space.
    if timezones.len() == 1 && xf > -179. && xf < 179. && yf > -89. && yf < 89. {
        return Some(timezones[0]);
    }

    timezones.into_iter().find(|&tz| tz.geometry.contains(&Coord { x: xf, y: yf }))
}

/// Get the exact timezone for a given longitude (x) and latitude (y).
#[allow(dead_code)]
fn get_timezone_via_full_lookup(xf: Float, yf: Float) -> Option<&'static NedTimezone> {
    NedTimezone::get_timezones().into_iter().find(|&tz| tz.geometry.contains(&Coord { x: xf, y: yf }))
}

/// Gets the geojson representation of the memory cache.
pub fn get_timezones_geojson() -> String {
    let geojson = NedTimezone::get_timezones().to_geojson();
    geojson.to_json_value().to_string()
}

/// Get value from the cache.
fn get_from_lookup(x: i16, y: i16) -> Option<Vec<&'static NedTimezone>> {
    let cache = NedTimezone::get_lookup();

    cache.get(&(x, y)).map_timezones()
}

// Trait impls.

impl HasLookupData for NedTimezone {
    fn get_timezones() -> &'static ConcreteVec<NedTimezone> {
        static TIMEZONES: OnceLock<ConcreteVec<NedTimezone>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            TIMEZONES.get_or_init(|| {
                let (timezones, _len): (ConcreteVec<NedTimezone>, usize) = bincode::serde::decode_from_slice(TZ_BINCODE, bincode::config::standard()).expect("Failed to decode timezones from binary: likely caused by precision difference between generated assets and current build.  Please rebuild the assets with `feature = \"force-rebuild\"` enabled.");

                timezones
            })
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::{
                shared::get_timezones_from_features,
                tz::ned::get_geojson_features_from_source,
            };

            TIMEZONES.get_or_init(|| {
                let features = get_geojson_features_from_source();
                get_timezones_from_features(features)
            })
        }
    }

    fn get_lookup() -> &'static HashMap<RoundLngLat, TimezoneIds> {
        static CACHE: OnceLock<HashMap<RoundLngLat, TimezoneIds>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            use rtz_core::geo::shared::RoundInt;

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
            use rtz_core::geo::shared::get_lookup_from_geometries;

            CACHE.get_or_init(|| {
                let cache = get_lookup_from_geometries(NedTimezone::get_timezones());

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
static TZ_BINCODE: &[u8] = include_bytes!("../../../../assets/ned_10m_time_zones.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\ned_10m_time_zones.bincode");

#[cfg(all(host_family_unix, feature = "self-contained"))]
static CACHE_BINCODE: &[u8] = include_bytes!("../../../../assets/ned_time_zone_lookup.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static CACHE_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\ned_time_zone_lookup.bincode");

// Tests.

#[cfg(test)]
mod tests {

    use super::super::shared::MapIntoTimezones;
    use super::*;
    use pretty_assertions::assert_eq;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    #[test]
    fn can_get_timezones() {
        let timezones = NedTimezone::get_timezones();
        assert_eq!(timezones.len(), 120);
    }

    #[test]
    fn can_get_lookup() {
        let cache = NedTimezone::get_lookup();
        assert_eq!(cache.len(), 64_800);
    }

    #[test]
    fn can_get_from_lookup() {
        let cache = get_from_lookup(-121, 46);
        assert_eq!(cache.unwrap().len(), 1);
    }

    #[test]
    fn can_perform_exact_lookup() {
        assert_eq!(get_timezone_via_full_lookup(-177.0, -15.0), None);
        assert_eq!(get_timezone_via_full_lookup(-121.0, 46.0).unwrap().identifier.as_ref().unwrap(), "America/Los_Angeles");

        assert_eq!(get_timezone_via_full_lookup(179.9968, -67.0959), None);
    }

    #[test]
    fn can_access_lookup() {
        let cache = NedTimezone::get_lookup();

        let tzs = cache.get(&(-177, -15)).map_timezones().unwrap() as Vec<&NedTimezone>;
        assert_eq!(tzs.len(), 2);

        let tzs = cache.get(&(-121, 46)).map_timezones().unwrap() as Vec<&NedTimezone>;
        assert_eq!(tzs.len(), 1);

        let tz = cache.get(&(-121, 46)).map_timezones().unwrap()[0] as &NedTimezone;
        assert_eq!(tz.identifier.as_ref().unwrap(), "America/Los_Angeles");

        let tzs = cache.get(&(-68, -67)).map_timezones().unwrap() as Vec<&NedTimezone>;
        assert_eq!(tzs.len(), 5);
    }

    #[test]
    fn can_verify_lookup_assisted_accuracy() {
        (0..1_000).into_par_iter().for_each(|_| {
            let x = rand::random::<Float>() * 360.0 - 180.0;
            let y = rand::random::<Float>() * 180.0 - 90.0;
            let full = get_timezone_via_full_lookup(x, y);
            let cache_assisted = get_timezone(x, y);

            assert_eq!(full.map(|t| t.id), cache_assisted.map(|t| t.id), "({}, {})", x, y);
        });
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use rtz_core::base::types::Float;
    use test::{black_box, Bencher};

    use super::{get_timezone, get_timezone_via_full_lookup};

    #[bench]
    fn bench_full_lookup_sweep(b: &mut Bencher) {
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezone_via_full_lookup(x as Float, y as Float));
                }
            }
        });
    }

    #[bench]
    fn bench_lookup_assisted_sweep(b: &mut Bencher) {
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezone(x as Float, y as Float));
                }
            }
        });
    }

    #[bench]
    fn bench_worst_case_full_lookup_single(b: &mut Bencher) {
        let x = -177;
        let y = -15;

        b.iter(|| {
            black_box(get_timezone_via_full_lookup(x as Float, y as Float));
        });
    }

    #[bench]
    fn bench_worst_case_lookup_assisted_single(b: &mut Bencher) {
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            black_box(get_timezone(x, y));
        });
    }
}
