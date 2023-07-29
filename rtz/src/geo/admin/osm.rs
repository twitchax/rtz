//! The [OpenStreetMap](https://www.openstreetmap.org/) admin lookup module.

use std::{collections::HashMap, sync::OnceLock};

use rtz_core::geo::{
    admin::{
        osm::OsmAdmin,
        shared::{i16_vec_to_adminids, AdminIds},
    },
    shared::{ConcreteVec, RoundLngLat},
};

use crate::geo::shared::{HasItemData, HasLookupData};

// Trait impls.

impl HasItemData for OsmAdmin {
    fn get_mem_items() -> &'static ConcreteVec<OsmAdmin> {
        static TIMEZONES: OnceLock<ConcreteVec<OsmAdmin>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            TIMEZONES.get_or_init(|| {
                let (timezones, _len): (ConcreteVec<OsmAdmin>, usize) = bincode::serde::decode_from_slice(ADMIN_BINCODE, bincode::config::standard()).expect("Failed to decode timezones from binary: likely caused by precision difference between generated assets and current build.  Please rebuild the assets with `feature = \"force-rebuild\"` enabled.");

                timezones
            })
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

impl HasLookupData for OsmAdmin {
    type Lookup = AdminIds;

    fn get_mem_lookup() -> &'static HashMap<RoundLngLat, Self::Lookup> {
        static CACHE: OnceLock<HashMap<RoundLngLat, AdminIds>> = OnceLock::new();

        #[cfg(feature = "self-contained")]
        {
            use rtz_core::geo::shared::RoundInt;

            CACHE.get_or_init(|| {
                let (cache, _len): (HashMap<RoundLngLat, Vec<RoundInt>>, usize) = bincode::serde::decode_from_slice(LOOKUP_BINCODE, bincode::config::standard()).unwrap();

                cache
                    .into_iter()
                    .map(|(key, value)| {
                        let value = i16_vec_to_adminids(value);

                        (key, value)
                    })
                    .collect::<HashMap<_, _>>()
            })
        }

        #[cfg(not(feature = "self-contained"))]
        {
            use rtz_core::geo::shared::get_lookup_from_geometries;

            CACHE.get_or_init(|| {
                let cache = get_lookup_from_geometries(OsmAdmin::get_items());

                cache
                    .into_iter()
                    .map(|(key, value)| {
                        let value = i16_vec_to_adminids(value);

                        (key, value)
                    })
                    .collect::<HashMap<_, _>>()
            })
        }
    }
}

// Statics.

#[cfg(all(host_family_unix, feature = "self-contained"))]
static ADMIN_BINCODE: &[u8] = include_bytes!("../../../../assets/osm_admins.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static ADMIN_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\osm_admins.bincode");

#[cfg(all(host_family_unix, feature = "self-contained"))]
static LOOKUP_BINCODE: &[u8] = include_bytes!("../../../../assets/osm_admin_lookup.bincode");
#[cfg(all(host_family_windows, feature = "self-contained"))]
static LOOKUP_BINCODE: &[u8] = include_bytes!("..\\..\\..\\..\\assets\\osm_admin_lookup.bincode");

// Tests.

// #[cfg(test)]
// mod tests {
//     use crate::geo::shared::{HasItemData, MapIntoItems};

//     use super::*;
//     use pretty_assertions::assert_eq;
//     use rayon::prelude::{IntoParallelIterator, ParallelIterator};

//     #[test]
//     fn can_get_timezones() {
//         let timezones = OsmTimezone::get_items();
//         assert_eq!(timezones.len(), 444);
//     }

//     #[test]
//     fn can_get_lookup() {
//         let cache = OsmTimezone::get_lookup();
//         assert_eq!(cache.len(), 64_800);
//     }

//     #[test]
//     fn can_get_from_lookup() {
//         let cache = get_from_lookup(-121, 46).unwrap();
//         assert_eq!(cache.len(), 1);
//     }

//     #[test]
//     fn can_perform_exact_lookup() {
//         assert_eq!(get_timezones_via_full_lookup(-177.0, -15.0).len(), 1);
//         assert_eq!(get_timezones_via_full_lookup(-121.0, 46.0)[0].identifier, "America/Los_Angeles");

//         assert_eq!(get_timezones_via_full_lookup(179.9968, -67.0959).len(), 1);
//     }

//     #[test]
//     fn can_access_lookup() {
//         let cache = OsmTimezone::get_lookup();

//         let tzs = cache.get(&(-177, -15)).map_into_items().unwrap() as Vec<&OsmTimezone>;
//         assert_eq!(tzs.len(), 1);

//         let tzs = cache.get(&(-121, 46)).map_into_items().unwrap() as Vec<&OsmTimezone>;
//         assert_eq!(tzs.len(), 1);

//         let tz = cache.get(&(-121, 46)).map_into_items().unwrap()[0] as &OsmTimezone;
//         assert_eq!(tz.identifier, "America/Los_Angeles");

//         let tzs = cache.get(&(-87, 38)).map_into_items().unwrap() as Vec<&OsmTimezone>;
//         assert_eq!(tzs.len(), 7);
//     }

//     #[test]
//     fn can_verify_lookup_assisted_accuracy() {
//         (0..100).into_par_iter().for_each(|_| {
//             let x = rand::random::<Float>() * 360.0 - 180.0;
//             let y = rand::random::<Float>() * 180.0 - 90.0;
//             let full = get_timezones_via_full_lookup(x, y);
//             let lookup_assisted = get_timezones(x, y);

//             assert_eq!(
//                 full.into_iter().map(|t| t.id).collect::<Vec<_>>(),
//                 lookup_assisted.into_iter().map(|t| t.id).collect::<Vec<_>>(),
//                 "({}, {})",
//                 x,
//                 y
//             );
//         });
//     }
// }

// #[cfg(test)]
// mod bench {
//     extern crate test;

//     use rtz_core::base::types::Float;
//     use test::{black_box, Bencher};

//     use super::*;

//     #[bench]
//     #[ignore]
//     fn bench_full_lookup_sweep(b: &mut Bencher) {
//         let xs = (-179..180).step_by(10);
//         let ys = (-89..90).step_by(10);

//         b.iter(|| {
//             for x in xs.clone() {
//                 for y in ys.clone() {
//                     black_box(get_timezones_via_full_lookup(x as Float, y as Float));
//                 }
//             }
//         });
//     }

//     #[bench]
//     fn bench_lookup_assisted_sweep(b: &mut Bencher) {
//         let xs = (-179..180).step_by(10);
//         let ys = (-89..90).step_by(10);

//         b.iter(|| {
//             for x in xs.clone() {
//                 for y in ys.clone() {
//                     black_box(get_timezones(x as Float, y as Float));
//                 }
//             }
//         });
//     }

//     #[bench]
//     fn bench_worst_case_full_lookup_single(b: &mut Bencher) {
//         let x = -86.5;
//         let y = 38.5;

//         b.iter(|| {
//             black_box(get_timezones_via_full_lookup(x as Float, y as Float));
//         });
//     }

//     #[bench]
//     fn bench_worst_case_lookup_assisted_single(b: &mut Bencher) {
//         let x = -86.5;
//         let y = 38.5;

//         b.iter(|| {
//             black_box(get_timezones(x, y));
//         });
//     }
// }
