//! The [Natural Earth Data](https://github.com/nvkelso/natural-earth-vector) timezone lookup module.

use std::{collections::HashMap, sync::OnceLock};

use geo::{Contains, Coord};
use rtz_core::{
    base::types::Float,
    geo::{
        shared::{ConcreteVec, EncodableIds, HasGeometry, RoundLngLat},
        tz::ned::NedTimezone,
    },
};

use crate::{
    geo::shared::{HasItemData, HasLookupData},
    CanPerformGeoLookup,
};

// Trait impls.

impl HasItemData for NedTimezone {
    fn get_mem_items() -> &'static ConcreteVec<NedTimezone> {
        static TIMEZONES: OnceLock<ConcreteVec<NedTimezone>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            TIMEZONES.get_or_init(|| crate::geo::shared::decode_binary_data(TZ_BINCODE))
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::{shared::get_items_from_features, tz::ned::get_geojson_features_from_source};

            TIMEZONES.get_or_init(|| {
                let features = get_geojson_features_from_source();
                get_items_from_features(features)
            })
        }
    }
}

impl HasLookupData for NedTimezone {
    type Lookup = EncodableIds;

    fn get_mem_lookup() -> &'static HashMap<RoundLngLat, Self::Lookup> {
        static CACHE: OnceLock<HashMap<RoundLngLat, EncodableIds>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            CACHE.get_or_init(|| crate::geo::shared::decode_binary_data(LOOKUP_BINCODE))
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::shared::get_lookup_from_geometries;

            CACHE.get_or_init(|| {
                let cache = get_lookup_from_geometries(NedTimezone::get_mem_items());

                cache
            })
        }
    }
}

// Special implementation of this for timezones since our timezone data covers the whole world.
// Therefore, we can use the special optimization.
impl CanPerformGeoLookup for NedTimezone {
    fn lookup(xf: Float, yf: Float) -> Vec<&'static Self> {
        let x = xf.floor() as i16;
        let y = yf.floor() as i16;

        let Some(suggestions) = Self::get_lookup_suggestions(x, y) else {
            return Vec::new();
        };

        // [ARoney] Optimization: If there is only one item, we can skip the more expensive
        // intersection check.  Edges are weird, so we still need to check if the point is in the
        // polygon at thg edges of the polar space.
        if suggestions.len() == 1 && xf > -179. && xf < 179. && yf > -89. && yf < 89. {
            return suggestions;
        }

        suggestions.into_iter().filter(|&i| i.geometry().contains(&Coord { x: xf, y: yf })).collect()
    }
}

// Statics.

#[cfg(all(host_family_unix, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("../../../../assets/ned_time_zones.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\ned_time_zones.bincode");

#[cfg(all(host_family_unix, feature = "self-contained"))]
static LOOKUP_BINCODE: &[u8] = include_bytes!("../../../../assets/ned_time_zone_lookup.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static LOOKUP_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\ned_time_zone_lookup.bincode");

// Tests.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::shared::{CanPerformGeoLookup, MapIntoItems};
    use pretty_assertions::assert_eq;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};
    use rtz_core::base::types::Float;

    #[test]
    fn can_get_timezones() {
        let timezones = NedTimezone::get_mem_items();
        assert_eq!(timezones.len(), 120);
    }

    #[test]
    fn can_get_lookup() {
        let cache = NedTimezone::get_mem_lookup();
        assert_eq!(cache.len(), 64_800);
    }

    #[test]
    fn can_get_from_lookup() {
        let cache = NedTimezone::get_lookup_suggestions(-121, 46);
        assert_eq!(cache.unwrap().len(), 1);
    }

    #[test]
    fn can_perform_exact_lookup() {
        assert_eq!(NedTimezone::lookup_slow(-177.0, -15.0).len(), 0);
        assert_eq!(NedTimezone::lookup_slow(-121.0, 46.0)[0].identifier.as_ref().unwrap(), "America/Los_Angeles");

        assert_eq!(NedTimezone::lookup_slow(179.9968, -67.0959).len(), 0);
    }

    #[test]
    fn can_access_lookup() {
        let cache = NedTimezone::get_mem_lookup();

        let tzs = cache.get(&(-177, -15)).map_into_items().unwrap() as Vec<&NedTimezone>;
        assert_eq!(tzs.len(), 2);

        let tzs = cache.get(&(-121, 46)).map_into_items().unwrap() as Vec<&NedTimezone>;
        assert_eq!(tzs.len(), 1);

        let tz = cache.get(&(-121, 46)).map_into_items().unwrap()[0] as &NedTimezone;
        assert_eq!(tz.identifier.as_ref().unwrap(), "America/Los_Angeles");

        let tzs = cache.get(&(-68, -67)).map_into_items().unwrap() as Vec<&NedTimezone>;
        assert_eq!(tzs.len(), 5);
    }

    #[test]
    fn can_verify_lookup_assisted_accuracy() {
        (0..1_000).into_par_iter().for_each(|_| {
            let x = rand::random::<Float>() * 360.0 - 180.0;
            let y = rand::random::<Float>() * 180.0 - 90.0;
            let full = NedTimezone::lookup_slow(x, y);
            let lookup_assisted = NedTimezone::lookup(x, y);

            assert_eq!(
                full.into_iter().map(|t| t.id).collect::<Vec<_>>(),
                lookup_assisted.into_iter().map(|t| t.id).collect::<Vec<_>>(),
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

    use crate::geo::shared::CanPerformGeoLookup;
    use rtz_core::{base::types::Float, geo::tz::ned::NedTimezone};
    use test::{black_box, Bencher};

    #[bench]
    fn bench_full_lookup_sweep(b: &mut Bencher) {
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(NedTimezone::lookup_slow(x as Float, y as Float));
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
                    black_box(NedTimezone::lookup(x as Float, y as Float));
                }
            }
        });
    }

    #[bench]
    fn bench_worst_case_full_lookup_single(b: &mut Bencher) {
        let x = -177;
        let y = -15;

        b.iter(|| {
            black_box(NedTimezone::lookup_slow(x as Float, y as Float));
        });
    }

    #[bench]
    fn bench_worst_case_lookup_assisted_single(b: &mut Bencher) {
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            black_box(NedTimezone::lookup(x, y));
        });
    }

    #[bench]
    fn bench_cities(b: &mut Bencher) {
        b.iter(|| {
            let city = cities_json::get_random_cities();
            black_box(NedTimezone::lookup(city.lng as f32, city.lat as f32));
        });
    }
}
