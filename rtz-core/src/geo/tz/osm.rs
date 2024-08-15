//! All of the geo-specific functions for OSM TZ lookups.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use std::{borrow::Cow, io::Read};

use geo::Geometry;
use serde_json::{Map, Value};

#[cfg(feature = "self-contained")]
use bincode::{
    de::{BorrowDecoder, Decoder},
    error::DecodeError,
    BorrowDecode, Decode, Encode,
};

use crate::{
    base::types::Float,
    geo::shared::{get_geojson_features_from_string, simplify_geometry, CanGetGeoJsonFeaturesFromSource, EncodableGeometry, EncodableString, HasGeometry, HasProperties, IdFeaturePair},
};

use super::shared::IsTimezone;

// Constants.

#[cfg(not(feature = "extrasimplified"))]
const SIMPLIFICATION_EPSILON: Float = 0.0001;
#[cfg(feature = "extrasimplified")]
const SIMPLIFICATION_EPSILON: Float = 0.01;

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
pub static ADDRESS: &str = "https://github.com/evansiroky/timezone-boundary-builder/releases/download/2024a/timezones-with-oceans.geojson.zip";
/// The name of the timezone bincode file.
pub static TIMEZONE_BINCODE_DESTINATION_NAME: &str = "osm_time_zones.bincode";
/// The name of the cache bincode file.
pub static LOOKUP_BINCODE_DESTINATION_NAME: &str = "osm_time_zone_lookup.bincode";

// Types.

/// A representation of the [OpenStreetMap](https://www.openstreetmap.org/)
/// [geojson](https://github.com/evansiroky/timezone-boundary-builder)
/// [`geojson::Feature`]s.
#[derive(Debug)]
#[cfg_attr(feature = "self-contained", derive(Encode))]
pub struct OsmTimezone {
    /// The index of the [`OsmTimezone`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The `identifier` of the [`OsmTimezone`] (e.g., `America/Los_Angeles`).
    ///
    /// Essentially, it is the IANA TZ identifier.
    pub identifier: EncodableString,

    /// The geometry of the [`OsmTimezone`].
    pub geometry: EncodableGeometry,
}

#[cfg(feature = "self-contained")]
impl Decode for OsmTimezone {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let id = usize::decode(decoder)?;
        let identifier = EncodableString::decode(decoder)?;
        let geometry = EncodableGeometry::decode(decoder)?;

        Ok(OsmTimezone { id, identifier, geometry })
    }
}

#[cfg(feature = "self-contained")]
impl<'de> BorrowDecode<'de> for OsmTimezone
where
    'de: 'static,
{
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let id = usize::decode(decoder)?;
        let identifier = EncodableString::borrow_decode(decoder)?;
        let geometry = EncodableGeometry::borrow_decode(decoder)?;

        Ok(OsmTimezone { id, identifier, geometry })
    }
}

impl PartialEq for OsmTimezone {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<IdFeaturePair> for OsmTimezone {
    fn from(value: IdFeaturePair) -> OsmTimezone {
        let id = value.0;
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let identifier = EncodableString(Cow::Owned(properties.get("tzid").unwrap().as_str().unwrap().to_string()));

        let geometry: Geometry<Float> = geometry.value.clone().try_into().unwrap();

        let geometry = EncodableGeometry(simplify_geometry(geometry, SIMPLIFICATION_EPSILON));

        OsmTimezone { id, identifier, geometry }
    }
}

impl IsTimezone for OsmTimezone {
    fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }
}

impl HasGeometry for OsmTimezone {
    fn id(&self) -> usize {
        self.id
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry.0
    }
}

impl HasProperties for OsmTimezone {
    fn properties(&self) -> Map<String, Value> {
        let mut properties = Map::new();

        properties.insert("identifier".to_string(), Value::String(self.identifier.to_string()));

        properties
    }
}

#[cfg(not(target_family = "wasm"))]
impl CanGetGeoJsonFeaturesFromSource for OsmTimezone {
    fn get_geojson_features_from_source() -> geojson::FeatureCollection {
        get_geojson_features_from_source()
    }
}
