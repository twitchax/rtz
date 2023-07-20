use std::collections::HashMap;

use chashmap::CHashMap;
use geo::{Contains, Coord, Intersects, Rect};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::sync::OnceLock;

use geojson::{FeatureCollection, GeoJson, JsonObject};

use super::types::{ConcreteTimezones, MapIntoTimezones, RoundLngLat, Timezone, TimezoneIds, TimezoneRef, TimezoneRefs};

// Constants.

const EPSILON: f64 = 0.01;

/// Get the cache-driven timezone for a given longitude (x) and latitude (y).
pub fn get_timezone(xf: f64, yf: f64) -> Option<TimezoneRef> {
    let x = xf.floor() as i16;
    let y = yf.floor() as i16;

    let cache = get_100km_cache();

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
pub fn get_from_100km_cache(x: i16, y: i16) -> Option<TimezoneRefs> {
    let cache = get_100km_cache();

    cache.get(&(x, y)).map_timezones()
}

/// Get the 100km cache.
fn get_100km_cache() -> &'static HashMap<RoundLngLat, TimezoneIds> {
    CACHE_100KM.get_or_init(|| {
        let json_str = CACHE_100KM_JSON;

        let json: JsonObject = serde_json::from_str(json_str).unwrap();

        let mut map = HashMap::new();
        for (key, value) in json.into_iter() {
            let tzs = value.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as i16).collect::<Vec<_>>();
            let value = [
                #[allow(clippy::get_first)]
                tzs.get(0).cloned().unwrap_or(-1),
                tzs.get(1).cloned().unwrap_or(-1),
                tzs.get(2).cloned().unwrap_or(-1),
                tzs.get(3).cloned().unwrap_or(-1),
                tzs.get(4).cloned().unwrap_or(-1),
                tzs.get(5).cloned().unwrap_or(-1),
                tzs.get(6).cloned().unwrap_or(-1),
                tzs.get(7).cloned().unwrap_or(-1),
                tzs.get(8).cloned().unwrap_or(-1),
                tzs.get(9).cloned().unwrap_or(-1),
            ];

            let key = key.split(',').map(|x| x.parse::<i16>().unwrap()).collect::<Vec<_>>();
            let key = (key[0], key[1]);

            map.insert(key, value);
        }

        map
    })
}

/// Get the timezones from the binary assets.
pub fn get_timezones() -> &'static ConcreteTimezones {
    TIMEZONES.get_or_init(|| {
        let features = get_geojson_features();
        ConcreteTimezones::from(features)
    })
}

/// Get the GeoJSON features from the binary assets.
pub fn get_geojson_features() -> &'static FeatureCollection {
    GEOJSON_FEATURECOLLECTION.get_or_init(|| FeatureCollection::try_from(TZ_GEOJSON.parse::<GeoJson>().unwrap()).unwrap())
}

/// Generate the JSON representation of the 100km cache.
///
/// "100km" is a bit of a misnomer.  This is really 100km _at the equator_, but this
/// makes it easier to reason about what the caches are doing.
pub async fn generate_100km_cache() {
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
                    intersected.push(tz.id);
                }
            }

            map.insert((x, y), intersected);
        }
    });

    let mut cache = HashMap::new();
    for (key, value) in map.into_iter() {
        cache.insert(format!("{},{}", key.0, key.1), value);
    }

    let json = serde_json::to_string(&cache).unwrap();
    std::fs::write("assets/100km_cache.json", json).unwrap();
}

// Statics.

static CACHE_100KM: OnceLock<HashMap<RoundLngLat, TimezoneIds>> = OnceLock::new();
static TIMEZONES: OnceLock<ConcreteTimezones> = OnceLock::new();
static GEOJSON_FEATURECOLLECTION: OnceLock<FeatureCollection> = OnceLock::new();

#[cfg(host_family_unix)]
static TZ_GEOJSON: &str = include_str!("../../assets/ne_10m_time_zones.geojson");
#[cfg(host_family_windows)]
static TZ_GEOJSON: &str = include_str!("..\\..\\assets\\ne_10m_time_zones.geojson");

#[cfg(host_family_unix)]
static CACHE_100KM_JSON: &str = include_str!("../../assets/100km_cache.json");
#[cfg(host_family_windows)]
static CACHE_100KM_JSON: &str = include_str!("..\\..\\assets\\100km_cache.json");

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
        let cache = get_100km_cache();
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
        let cache = get_100km_cache();

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
