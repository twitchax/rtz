//! All of the geo-specific functions for OSM TZ lookups.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so 
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use geo::{algorithm::simplify_vw::SimplifyVw, Geometry};
use serde::{Deserialize, Serialize};

use crate::base::types::Float;

use super::shared::IsTimezone;

// Statics.

const SIMPLIFICATION_EPSILON: Float = 0.0001;

/// The address of the GeoJSON file.
pub static GEOJSON_ADDRESS: &str = "https://github.com/evansiroky/timezone-boundary-builder/releases/download/2023b/timezones-with-oceans.geojson.zip";
/// The name of the timezone bincode file.
pub static TIMEZONE_BINCODE_DESTINATION_NAME: &str = "osm_time_zones.bincode";
/// The name of the cache bincode file.
pub static CACHE_BINCODE_DESTINATION_NAME: &str = "osm_time_zone_cache.bincode";

// Types.

/// A representation of the [OpenStreetMap](https://www.openstreetmap.org/)
/// [geojson](https://github.com/evansiroky/timezone-boundary-builder)
/// [`geojson::Feature`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct OsmTimezone {
    /// The index of the [`OsmTimezone`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The `identifier` of the [`OsmTimezone`] (e.g., `America/Los_Angeles`).
    ///
    /// Essentially, it is the IANA TZ identifier.
    pub identifier: String,

    /// The geometry of the [`OsmTimezone`].
    pub geometry: Geometry<Float>,
}

impl PartialEq for OsmTimezone {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<(usize, geojson::Feature)> for OsmTimezone {
    fn from(value: (usize, geojson::Feature)) -> OsmTimezone {
        let id = value.0;
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let identifier = properties.get("tzid").unwrap().as_str().unwrap().to_string();

        let geometry: Geometry<Float> = geometry.value.clone().try_into().unwrap();

        #[cfg(not(feature = "unsimplified"))]
        let geometry = match geometry {
            Geometry::Polygon(polygon) => {
                let simplified = polygon.simplify_vw(&SIMPLIFICATION_EPSILON);
                Geometry::Polygon(simplified)
            }
            Geometry::MultiPolygon(multi_polygon) => {
                let simplified = multi_polygon.simplify_vw(&SIMPLIFICATION_EPSILON);
                Geometry::MultiPolygon(simplified)
            }
            _ => panic!("Unexpected geometry type!"),
        };

        OsmTimezone { id, identifier, geometry }
    }
}

impl IsTimezone for OsmTimezone {
    fn id(&self) -> usize {
        self.id
    }

    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry
    }
}
