//! All of the geo-specific functions for OSM TZ lookups.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use std::io::Read;

use geo::Geometry;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    base::types::Float,
    geo::shared::{simplify_geometry, HasGeometry, HasProperties, get_geojson_features_from_string},
};

use super::shared::IsTimezone;

// Helpers.

/// Get the GeoJSON [`geojson::Feature`]s from the source.
#[cfg(not(target_family = "wasm"))]
pub fn get_geojson_features_from_source() -> geojson::FeatureCollection {
    let response = reqwest::blocking::get(ADDRESS).unwrap();
    let geojson_zip = response.bytes().unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(geojson_zip)).unwrap();
    let mut geojson_input = String::new();
    zip.by_index(0).unwrap().read_to_string(&mut geojson_input).unwrap();

    get_geojson_features_from_string(&geojson_input)
}

/// The address of the GeoJSON file.
pub static ADDRESS: &str = "https://github.com/evansiroky/timezone-boundary-builder/releases/download/2023b/timezones-with-oceans.geojson.zip";
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

        let geometry = simplify_geometry(geometry);

        OsmTimezone { id, identifier, geometry }
    }
}

impl IsTimezone for OsmTimezone {
    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }
}

impl HasGeometry for OsmTimezone {
    fn id(&self) -> usize {
        self.id
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry
    }
}

impl HasProperties for OsmTimezone {
    fn properties(&self) -> Map<String, Value> {
        let mut properties = Map::new();

        properties.insert("identifier".to_string(), Value::String(self.identifier.to_string()));

        properties
    }
}
