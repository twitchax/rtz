//! All of the geo-specific functions for OSM admin lookups.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use geo::Geometry;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    base::types::Float,
    geo::shared::{get_geojson_feature_from_string, simplify_geometry, HasGeometry, HasProperties},
};

use super::shared::IsAdmin;

// Helpers.

/// Get the GeoJSON [`geojson::Feature`]s from the source.
#[cfg(not(target_family = "wasm"))]
pub fn get_geojson_features_from_source() -> geojson::FeatureCollection {
    let files = std::fs::read_dir(ADDRESS)
        .unwrap()
        .filter(|f| f.as_ref().unwrap().file_name().to_str().unwrap().ends_with(".geojson"))
        .map(|f| f.unwrap())
        .collect::<Vec<_>>();

    let mut collection = geojson::FeatureCollection {
        bbox: None,
        features: Vec::new(),
        foreign_members: None,
    };

    for file in files {
        let json = std::fs::read_to_string(file.path()).unwrap();

        let feature = get_geojson_feature_from_string(&json);

        collection.features.push(feature);
    }

    collection
}

/// The address of the GeoJSON file.
pub static ADDRESS: &str = "D://LargeData//admin_data//admin2";
/// The name of the timezone bincode file.
pub static ADMIN_BINCODE_DESTINATION_NAME: &str = "osm_admins.bincode";
/// The name of the cache bincode file.
pub static LOOKUP_BINCODE_DESTINATION_NAME: &str = "osm_admin_lookup.bincode";

// Types.

/// A representation of the [OpenStreetMap](https://www.openstreetmap.org/)
/// [geojson](https://github.com/evansiroky/timezone-boundary-builder)
/// [`geojson::Feature`]s for administrative areas.
#[derive(Debug, Serialize, Deserialize)]
pub struct OsmAdmin {
    /// The index of the [`OsmAdmin`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,

    /// The `name` of the [`OsmAdmin`] (e.g., `Burkina Faso`).
    pub name: String,
    /// The `level` of the [`OsmAdmin`] (e.g., `3`).
    pub level: u8,

    /// The geometry of the [`OsmAdmin`].
    pub geometry: Geometry<Float>,
}

impl PartialEq for OsmAdmin {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<(usize, geojson::Feature)> for OsmAdmin {
    fn from(value: (usize, geojson::Feature)) -> OsmAdmin {
        let id = value.0;
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let name = properties.get("name").unwrap().as_str().unwrap().to_string();
        let level = properties.get("admin_level").unwrap().as_u64().unwrap() as u8;

        let geometry: Geometry<Float> = geometry.value.clone().try_into().unwrap();
        let geometry = simplify_geometry(geometry);

        OsmAdmin { id, name, level, geometry }
    }
}

impl IsAdmin for OsmAdmin {
    fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl HasGeometry for OsmAdmin {
    fn id(&self) -> usize {
        self.id
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry
    }
}

impl HasProperties for OsmAdmin {
    fn properties(&self) -> Map<String, Value> {
        let mut properties = Map::new();

        properties.insert("name".to_string(), Value::String(self.name.to_string()));
        properties.insert("level".to_string(), Value::String(self.level.to_string()));

        properties
    }
}
