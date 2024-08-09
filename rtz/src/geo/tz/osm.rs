//! The [OpenStreetMap](https://www.openstreetmap.org/) timezone lookup module.

use std::{collections::HashMap, sync::OnceLock};

use geo::{Contains, Coord};
use rtz_core::{
    base::types::Float,
    geo::{
        shared::{ConcreteVec, EncodableIds, HasGeometry, RoundLngLat},
        tz::osm::OsmTimezone,
    },
};

use crate::{
    geo::shared::{HasItemData, HasLookupData},
    CanPerformGeoLookup,
};

// Trait impls.

impl HasItemData for OsmTimezone {
    fn get_mem_items() -> &'static ConcreteVec<OsmTimezone> {
        static TIMEZONES: OnceLock<ConcreteVec<OsmTimezone>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            TIMEZONES.get_or_init(|| crate::geo::shared::decode_binary_data(TZ_BINCODE))
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::{shared::get_items_from_features, tz::osm::get_geojson_features_from_source};

            TIMEZONES.get_or_init(|| {
                let features = get_geojson_features_from_source();

                get_items_from_features(features)
            })
        }
    }
}

impl HasLookupData for OsmTimezone {
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
                let cache = get_lookup_from_geometries(OsmTimezone::get_mem_items());

                cache
            })
        }
    }
}

// Special implementation of this for timezones since our timezone data covers the whole world.
// Therefore, we can use the special optimization.
impl CanPerformGeoLookup for OsmTimezone {
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
static TZ_BINCODE: &[u8] = include_bytes!("../../../../assets/osm_time_zones.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static TZ_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\osm_time_zones.bincode");

#[cfg(all(host_family_unix, feature = "self-contained"))]
static LOOKUP_BINCODE: &[u8] = include_bytes!("../../../../assets/osm_time_zone_lookup.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static LOOKUP_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\osm_time_zone_lookup.bincode");

// Tests.

#[cfg(test)]
mod tests {
    use crate::geo::shared::{CanPerformGeoLookup, HasItemData, MapIntoItems};

    use super::*;
    use pretty_assertions::assert_eq;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};
    use rtz_core::base::types::Float;

    #[test]
    fn can_get_timezones() {
        let timezones = OsmTimezone::get_mem_items();
        assert_eq!(timezones.len(), 444);
    }

    #[test]
    fn can_get_lookup() {
        let cache = OsmTimezone::get_mem_lookup();
        assert_eq!(cache.len(), 64_800);
    }

    #[test]
    fn can_get_from_lookup() {
        let cache = OsmTimezone::get_lookup_suggestions(-121, 46).unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn can_perform_exact_lookup() {
        assert_eq!(OsmTimezone::lookup_slow(-177.0, -15.0).len(), 1);
        assert_eq!(OsmTimezone::lookup_slow(-121.0, 46.0)[0].identifier, "America/Los_Angeles");

        assert_eq!(OsmTimezone::lookup_slow(179.9968, -67.0959).len(), 1);
    }

    #[test]
    fn can_access_lookup() {
        let cache = OsmTimezone::get_mem_lookup();

        let tzs = cache.get(&(-177, -15)).map_into_items().unwrap() as Vec<&OsmTimezone>;
        assert_eq!(tzs.len(), 1);

        let tzs = cache.get(&(-121, 46)).map_into_items().unwrap() as Vec<&OsmTimezone>;
        assert_eq!(tzs.len(), 1);

        let tz = cache.get(&(-121, 46)).map_into_items().unwrap()[0] as &OsmTimezone;
        assert_eq!(tz.identifier, "America/Los_Angeles");

        let tzs = cache.get(&(-87, 38)).map_into_items().unwrap() as Vec<&OsmTimezone>;
        assert_eq!(tzs.len(), 7);
    }

    #[test]
    fn can_verify_lookup_assisted_accuracy() {
        let x = rand::random::<Float>() * 360.0 - 180.0;
        (0..100).into_par_iter().for_each(|_| {
            let y = rand::random::<Float>() * 180.0 - 90.0;
            let full = OsmTimezone::lookup_slow(x, y);
            let lookup_assisted = OsmTimezone::lookup(x, y);

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