//! All of the geo-specific functions.

use std::collections::HashMap;

use chashmap::CHashMap;
use geo::{Contains, Coord, Intersects, Rect};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::sync::OnceLock;

use geojson::{FeatureCollection, GeoJson};

use super::types::{ConcreteTimezones, MapIntoTimezones, RoundLngLat, TimezoneIds, TimezoneRef, TimezoneRefs, RoundInt};

// Constants.

/// Get the cache-driven timezone for a given longitude (x) and latitude (y).
pub fn get_timezone(xf: f64, yf: f64) -> Option<TimezoneRef> {
    let x = xf.floor() as i16;
    let y = yf.floor() as i16;

    let cache = get_cache();

    let timezones = cache.get(&(x, y)).map_timezones()?;

    // [ARoney] Optimization: If there is only one timezone, we can skip the more expensive
    // intersection check.  Edges are weird, so we still need to check if the point is in the
    // polygon at thg edges of the polar space.
    if timezones.len() == 1 && xf > -179. && xf < 179. && yf > -89. && yf < 89. {
        return Some(timezones[0]);
    }

    timezones.into_iter().find(|&tz| tz.geometry.contains(&Coord { x: xf, y: yf }))
}

/// Get the exact timezone for a given longitude (x) and latitude (y).
pub fn get_timezone_via_full_lookup(xf: f64, yf: f64) -> Option<TimezoneRef> {
    get_timezones().into_iter().find(|&tz| tz.geometry.contains(&Coord { x: xf, y: yf }))
}

/// Get value from the 100km cache.
fn get_from_cache(x: i16, y: i16) -> Option<TimezoneRefs> {
    let cache = get_cache();

    cache.get(&(x, y)).map_timezones()
}

/// Get the 100km cache.
fn get_cache() -> &'static HashMap<RoundLngLat, TimezoneIds> {
    CACHE.get_or_init(|| {
        let (cache, _len): (HashMap<RoundLngLat, Vec<RoundInt>>, usize) = bincode::serde::decode_from_slice(CACHE_BINCODE, bincode::config::standard()).unwrap();

        cache.into_iter().map(|(key, value)| {
            let value = [
                #[allow(clippy::get_first)]
                value.get(0).cloned().unwrap_or(-1),
                value.get(1).cloned().unwrap_or(-1),
                value.get(2).cloned().unwrap_or(-1),
                value.get(3).cloned().unwrap_or(-1),
                value.get(4).cloned().unwrap_or(-1),
                value.get(5).cloned().unwrap_or(-1),
                value.get(6).cloned().unwrap_or(-1),
                value.get(7).cloned().unwrap_or(-1),
                value.get(8).cloned().unwrap_or(-1),
                value.get(9).cloned().unwrap_or(-1),
            ];

            (key, value)
        }).collect::<HashMap<_, _>>()
    })
}

/// Get the timezones from the binary assets.
pub(crate) fn get_timezones() -> &'static ConcreteTimezones {
    TIMEZONES.get_or_init(|| {
        let (concrete_timezones, _len): (ConcreteTimezones, usize) = bincode::serde::decode_from_slice(TZ_BINCODE, bincode::config::standard()).unwrap();
        concrete_timezones
    })
}

/// Get the GeoJSON features from the binary assets.
fn get_geojson_features() -> &'static FeatureCollection {
    GEOJSON_FEATURECOLLECTION.get_or_init(|| {
        let tz_geojson = std::fs::read_to_string("assets/ne_10m_time_zones.geojson").unwrap();
        FeatureCollection::try_from(tz_geojson.parse::<GeoJson>().unwrap()).unwrap()
    })
}

/// Generate the bincode representation of the 100km cache.
///
/// "100km" is a bit of a misnomer.  This is really 100km _at the equator_, but this
/// makes it easier to reason about what the caches are doing.
fn generate_100km_cache() {
    let timezones = get_timezones();
    let map = CHashMap::new();

    (-180..180).into_par_iter().for_each(|x| {
        for y in -90..90 {
            let xf = x as f64;
            let yf = y as f64;

            let rect = Rect::new(Coord { x: xf, y: yf }, Coord { x: xf + 1.0, y: yf + 1.0 });

            let mut intersected = Vec::new();

            for tz in timezones {
                if tz.geometry.intersects(&rect) {
                    intersected.push(tz.id as RoundInt);
                }
            }

            map.insert((x as RoundInt, y as RoundInt), intersected);
        }
    });

    let mut cache = HashMap::new();
    for (key, value) in map.into_iter() {
        cache.insert(key, value);
    }

    std::fs::write("assets/100km_cache.bincode", bincode::serde::encode_to_vec(&cache, bincode::config::standard()).unwrap()).unwrap();

    //let json = serde_json::to_string(&cache).unwrap();
    //std::fs::write("assets/100km_cache.json", json).unwrap();
}

/// Generate bincode representation of the timezones.
fn generate_timezones() {
    let features = get_geojson_features();
    let timezones = ConcreteTimezones::from(features);

    std::fs::write("assets/ne_10m_time_zones.bincode", bincode::serde::encode_to_vec(timezones, bincode::config::standard()).unwrap()).unwrap();
}

/// Generates new bincodes for the timezones and the cache from the GeoJSON.
pub(crate) fn generate_bincodes() {
    generate_timezones();
    generate_100km_cache();
}

// Statics.

static CACHE: OnceLock<HashMap<RoundLngLat, TimezoneIds>> = OnceLock::new();
static TIMEZONES: OnceLock<ConcreteTimezones> = OnceLock::new();
static GEOJSON_FEATURECOLLECTION: OnceLock<FeatureCollection> = OnceLock::new();

#[cfg(host_family_unix)]
static TZ_BINCODE: &[u8] = include_bytes!("../../assets/ne_10m_time_zones.bincode");
#[cfg(host_family_windows)]
static TZ_BINCODE: &[u8] = include_bytes!("..\\..\\assets\\ne_10m_time_zones.bincode");

#[cfg(host_family_unix)]
static CACHE_100KM_BINCODE: &[u8] = include_bytes!("../../assets/100km_cache.bincode");
#[cfg(host_family_windows)]
static CACHE_BINCODE: &[u8] = include_bytes!("..\\..\\assets\\100km_cache.bincode");

// Tests.

#[cfg(test)]
mod tests {
    use crate::base::types::MapIntoTimezones;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_get_timezones() {
        let timezones = get_timezones();
        assert_eq!(timezones.len(), 120);
    }

    #[test]
    fn test_get_100km_cache() {
        let cache = get_cache();
        assert_eq!(cache.len(), 64_800);
    }

    #[test]
    fn test_exact_lookup() {
        assert_eq!(get_timezone_via_full_lookup(-177.0, -15.0), None);
        assert_eq!(get_timezone_via_full_lookup(-121.0, 46.0).unwrap().friendly_name.as_ref().unwrap(), "America/Los_Angeles");

        assert_eq!(get_timezone_via_full_lookup(179.9968, -67.0959), None);
    }

    #[test]
    fn test_100km_cache() {
        let cache = get_cache();

        assert_eq!(cache.get(&(-177, -15)).map_timezones().unwrap().len(), 2);

        assert_eq!(cache.get(&(-121, 46)).map_timezones().unwrap().len(), 1);
        assert_eq!(cache.get(&(-121, 46)).map_timezones().unwrap()[0].friendly_name.as_ref().unwrap(), "America/Los_Angeles");

        assert_eq!(cache.get(&(-68, -67)).map_timezones().unwrap().len(), 5);
    }

    #[test]
    fn test_cache_assisted_accuracy() {
        (0..10_000).into_par_iter().for_each(|_| {
            let x = rand::random::<f64>() * 360.0 - 180.0;
            let y = rand::random::<f64>() * 180.0 - 90.0;
            let full = get_timezone_via_full_lookup(x, y);
            let cache_assisted = get_timezone(x, y);

            assert_eq!(full.map(|t| t.id), cache_assisted.map(|t| t.id), "({}, {})", x, y);
        });
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use test::{black_box, Bencher};

    use crate::base::geo::{get_timezone, get_timezone_via_full_lookup};

    #[bench]
    fn bench_full_lookup_sweep(b: &mut Bencher) {
        // Optionally include some setup.
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezone_via_full_lookup(x as f64, y as f64));
                }
            }
        });
    }

    #[bench]
    fn bench_cache_assisted_sweep(b: &mut Bencher) {
        // Optionally include some setup.
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezone(x as f64, y as f64));
                }
            }
        });
    }

    #[bench]
    fn bench_worst_case_full_lookup_single(b: &mut Bencher) {
        // Optionally include some setup.
        let x = -177;
        let y = -15;

        b.iter(|| {
            black_box(get_timezone_via_full_lookup(x as f64, y as f64));
        });
    }

    #[bench]
    fn bench_worst_case_cache_assisted_single(b: &mut Bencher) {
        // Optionally include some setup.
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            black_box(get_timezone(x, y));
        });
    }
}