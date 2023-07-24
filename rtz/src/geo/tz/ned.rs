//! The [Natural Earth Data](https://github.com/nvkelso/natural-earth-vector) timezone lookup module.

use std::{collections::HashMap, sync::OnceLock};

use geo::{Coord, Contains};
use rtz_core::{geo::tz::ned::{TimezoneRef, TimezoneRefs, RoundLngLat, TimezoneIds, ConcreteTimezones}, base::types::Res};


/// Get the cache-driven timezone for a given longitude (x) and latitude (y).
pub fn get_timezone(xf: f64, yf: f64) -> Option<TimezoneRef> {
    let x = xf.floor() as i16;
    let y = yf.floor() as i16;

    let timezones = get_from_cache(x, y)?;

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
pub(crate) fn get_from_cache(x: i16, y: i16) -> Option<TimezoneRefs> {
    let cache = get_cache();

    cache.get(&(x, y)).map_timezones()
}

/// Get the 100km cache.
fn get_cache() -> &'static HashMap<RoundLngLat, TimezoneIds> {
    static CACHE: OnceLock<HashMap<RoundLngLat, TimezoneIds>> = OnceLock::new();

    #[cfg(feature = "self-contained")]
    {
        use rtz_core::geo::tz::ned::RoundInt;
        
        CACHE.get_or_init(|| {
            let (cache, _len): (HashMap<RoundLngLat, Vec<RoundInt>>, usize) = bincode::serde::decode_from_slice(CACHE_BINCODE, bincode::config::standard()).unwrap();
    
            cache
                .into_iter()
                .map(|(key, value)| {
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
                })
                .collect::<HashMap<_, _>>()
        })
    }

    #[cfg(not(feature = "self-contained"))]
    {
        use rtz_core::geo::tz::ned::get_cache_from_timezones;

        CACHE.get_or_init(|| {
            let cache = get_cache_from_timezones(get_timezones());

            cache
                .into_iter()
                .map(|(key, value)| {
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
                })
                .collect::<HashMap<_, _>>()
        })
    }
}

/// Get the timezones from the binary assets.
pub(crate) fn get_timezones() -> &'static ConcreteTimezones {
    #[cfg(feature = "self-contained")]
    {
        static TIMEZONES: OnceLock<ConcreteTimezones> = OnceLock::new();

        TIMEZONES.get_or_init(|| {
            let (timezones, _len): (ConcreteTimezones, usize) = bincode::serde::decode_from_slice(TZ_BINCODE, bincode::config::standard()).unwrap();

            timezones
        })
    }

    #[cfg(not(feature = "self-contained"))]
    {
        use rtz_core::geo::tz::ned::{GEOJSON_ADDRESS, get_geojson_features_from_string, get_timezones_from_features};
        
        static TIMEZONES: OnceLock<ConcreteTimezones> = OnceLock::new();

        TIMEZONES.get_or_init(|| {
            let response = reqwest::blocking::get(GEOJSON_ADDRESS).unwrap();
            let geojson_input = response.text().unwrap();

            let features = get_geojson_features_from_string(&geojson_input);
            
            get_timezones_from_features(features)
        })
    }
}

// Statics.

#[cfg(all(host_family_unix, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("../../../../assets/ne_10m_time_zones.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\ne_10m_time_zones.bincode");

#[cfg(all(host_family_unix, feature = "self-contained"))]
static CACHE_BINCODE: &[u8] = include_bytes!("../../../../assets/ne_time_zone_cache.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static CACHE_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\ne_time_zone_cache.bincode");

// Types.

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait IntoTimezone {
    fn into_timezone(self) -> Res<TimezoneRef>;
}

impl IntoTimezone for u16 {
    fn into_timezone(self) -> Res<TimezoneRef> {
        Some(&self).map_timezone().ok_or_else(|| anyhow::Error::msg("Timezone not found."))
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait MapIntoTimezone {
    fn map_timezone(self) -> Option<TimezoneRef>;
}

impl MapIntoTimezone for Option<&u16> {
    fn map_timezone(self) -> Option<TimezoneRef> {
        let Some(value) = self else {
            return None;
        };

        let timezones = get_timezones();

        timezones.get(*value as usize)
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait MapIntoTimezones {
    fn map_timezones(self) -> Option<TimezoneRefs>;
}

impl MapIntoTimezones for Option<&TimezoneIds> {
    fn map_timezones(self) -> Option<TimezoneRefs> {
        let Some(value) = self else {
            return None;
        };

        let timezones = get_timezones();

        let mut result = Vec::with_capacity(10);
        for id in value {
            if *id == -1 {
                continue;
            }

            let tz = timezones.get(*id as usize);

            if let Some(tz) = tz {
                result.push(tz);
            }
        }

        Some(result)
    }
}

// Tests.

#[cfg(test)]
mod tests {

    use super::*;
    use pretty_assertions::assert_eq;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    #[test]
    fn can_get_timezones() {
        let timezones = get_timezones();
        assert_eq!(timezones.len(), 120);
    }

    #[test]
    fn can_get_cache() {
        let cache = get_cache();
        assert_eq!(cache.len(), 64_800);
    }

    #[test]
    fn can_get_from_cache() {
        let cache = get_from_cache(-121, 46);
        assert_eq!(cache.unwrap().len(), 1);
    }

    #[test]
    fn can_perform_exact_lookup() {
        assert_eq!(get_timezone_via_full_lookup(-177.0, -15.0), None);
        assert_eq!(get_timezone_via_full_lookup(-121.0, 46.0).unwrap().friendly_name.as_ref().unwrap(), "America/Los_Angeles");

        assert_eq!(get_timezone_via_full_lookup(179.9968, -67.0959), None);
    }

    #[test]
    fn can_access_100km_cache() {
        let cache = get_cache();

        assert_eq!(cache.get(&(-177, -15)).map_timezones().unwrap().len(), 2);

        assert_eq!(cache.get(&(-121, 46)).map_timezones().unwrap().len(), 1);
        assert_eq!(cache.get(&(-121, 46)).map_timezones().unwrap()[0].friendly_name.as_ref().unwrap(), "America/Los_Angeles");

        assert_eq!(cache.get(&(-68, -67)).map_timezones().unwrap().len(), 5);
    }

    #[test]
    fn can_verify_cache_assisted_accuracy() {
        (0..1_000).into_par_iter().for_each(|_| {
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

    use super::{get_timezone_via_full_lookup, get_timezone};

    #[bench]
    fn bench_full_lookup_sweep(b: &mut Bencher) {
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
        let x = -177;
        let y = -15;

        b.iter(|| {
            black_box(get_timezone_via_full_lookup(x as f64, y as f64));
        });
    }

    #[bench]
    fn bench_worst_case_cache_assisted_single(b: &mut Bencher) {
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            black_box(get_timezone(x, y));
        });
    }
}
