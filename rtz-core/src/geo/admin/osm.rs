//! All of the geo-specific functions for OSM admin lookups.

use geo::Geometry;
use serde_json::{Map, Value};
use std::borrow::Cow;

#[cfg(feature = "self-contained")]
use bincode::{
    de::{BorrowDecoder, Decoder},
    error::DecodeError,
    BorrowDecode, Decode, Encode,
};

use crate::{
    base::types::Float,
    geo::shared::{simplify_geometry, EncodableGeometry, EncodableString, HasGeometry, HasProperties, IdFeaturePair},
};

// Source ingestion is native-only (it reads GeoJSON off disk), so its imports carry the same
// gate as the functions that use them.
#[cfg(not(target_family = "wasm"))]
use crate::geo::shared::{get_geojson_feature_from_string, CanGetGeoJsonFeaturesFromSource};

use super::shared::IsAdmin;

// Constants.

#[cfg(not(feature = "extrasimplified"))]
const SIMPLIFICATION_EPSILON: Float = 0.001;
#[cfg(feature = "extrasimplified")]
const SIMPLIFICATION_EPSILON: Float = 0.1;

// Helpers.

/// Get the GeoJSON [`geojson::Feature`]s from the source.
#[cfg(not(target_family = "wasm"))]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_geojson_features_from_source() -> geojson::FeatureCollection {
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    let admin_dirs = std::env::var("RTZ_OSM_ADMIN_DIRS")
        .expect("RTZ_OSM_ADMIN_DIRS must be set (semicolon-separated admin GeoJSON directories) to regenerate OSM admin data");
    let paths = admin_dirs.split(';').collect::<Vec<_>>();
    let mut files = Vec::new();

    for path in paths {
        let mut path_files = std::fs::read_dir(path)
            .unwrap()
            .filter(|f| f.as_ref().unwrap().file_name().to_str().unwrap().ends_with(".geojson"))
            .map(|f| f.unwrap())
            .collect::<Vec<_>>();

        files.append(&mut path_files);
    }

    let features = files
        .into_par_iter()
        .filter(|f| {
            let md = f.metadata().unwrap();

            md.len() != 0
        })
        .map(|f| {
            let json = std::fs::read_to_string(f.path()).unwrap();
            get_geojson_feature_from_string(&json)
        })
        .collect::<Vec<_>>();

    geojson::FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

/// The name of the timezone bincode file.
pub static ADMIN_BINCODE_DESTINATION_NAME: &str = "osm_admins.bincode";
/// The name of the cache bincode file.
pub static LOOKUP_BINCODE_DESTINATION_NAME: &str = "osm_admin_lookup.bincode";

// Types.

/// A representation of the [OpenStreetMap](https://www.openstreetmap.org/)
/// [geojson](https://github.com/evansiroky/timezone-boundary-builder)
/// [`geojson::Feature`]s for administrative areas.
#[derive(Debug)]
#[cfg_attr(feature = "self-contained", derive(Encode))]
pub struct OsmAdmin {
    /// The index of the [`OsmAdmin`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,

    /// The OSM relation id of the admin area (e.g., `1473947`), or `0` if the source boundary was
    /// not relation-backed. Unlike [`OsmAdmin::id`], this is stable across builds and data sets.
    pub relation_id: u64,

    /// The `name` of the [`OsmAdmin`] (e.g., `Burkina Faso`).
    pub name: EncodableString,
    /// The `level` of the [`OsmAdmin`] (e.g., `3`).
    pub level: usize,

    /// The geometry of the [`OsmAdmin`].
    pub geometry: EncodableGeometry,
}

#[cfg(feature = "self-contained")]
impl<Context> Decode<Context> for OsmAdmin {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder<Context = Context>,
    {
        let id = usize::decode(decoder)?;
        let relation_id = u64::decode(decoder)?;
        let name = EncodableString::decode(decoder)?;
        let level = usize::decode(decoder)?;
        let geometry = EncodableGeometry::decode(decoder)?;

        Ok(OsmAdmin { id, relation_id, name, level, geometry })
    }
}

#[cfg(feature = "self-contained")]
impl<'de, Context> BorrowDecode<'de, Context> for OsmAdmin
where
    'de: 'static,
{
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de, Context = Context>,
    {
        let id = usize::decode(decoder)?;
        let relation_id = u64::decode(decoder)?;
        let name = EncodableString::borrow_decode(decoder)?;
        let level = usize::decode(decoder)?;
        let geometry = EncodableGeometry::borrow_decode(decoder)?;

        Ok(OsmAdmin { id, relation_id, name, level, geometry })
    }
}

impl PartialEq for OsmAdmin {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<IdFeaturePair> for OsmAdmin {
    fn from(value: IdFeaturePair) -> OsmAdmin {
        let id = value.0;
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        // Read defensively: a way-backed boundary can lack `relation_id`, and a single missing
        // value would otherwise panic the whole regen. `0` marks "unknown" (no OSM id is ever 0).
        let relation_id = properties.get("relation_id").and_then(|v| v.as_u64()).unwrap_or(0);
        let name = EncodableString(Cow::Owned(properties.get("name").unwrap().as_str().unwrap().to_string()));
        let level = properties.get("admin_level").unwrap().as_u64().unwrap() as usize;

        let geometry: Geometry<Float> = geometry.value.clone().try_into().unwrap();
        let geometry = EncodableGeometry(simplify_geometry(geometry, SIMPLIFICATION_EPSILON));

        OsmAdmin { id, relation_id, name, level, geometry }
    }
}

impl IsAdmin for OsmAdmin {
    fn name(&self) -> &str {
        self.name.as_ref()
    }
}

impl HasGeometry for OsmAdmin {
    fn id(&self) -> usize {
        self.id
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry.0
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

#[cfg(not(target_family = "wasm"))]
impl CanGetGeoJsonFeaturesFromSource for OsmAdmin {
    fn get_geojson_features_from_source() -> geojson::FeatureCollection {
        get_geojson_features_from_source()
    }
}
