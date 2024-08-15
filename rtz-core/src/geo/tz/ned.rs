//! All of the geo-specific functions for NED TZ lookups.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use std::borrow::Cow;

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
    geo::shared::{
        get_geojson_features_from_string, simplify_geometry, CanGetGeoJsonFeaturesFromSource, EncodableGeometry, EncodableOptionString, EncodableString, HasGeometry, HasProperties, IdFeaturePair,
    },
};

use super::shared::IsTimezone;

// Constants.

#[cfg(not(feature = "extrasimplified"))]
const SIMPLIFICATION_EPSILON: Float = 0.00001;
#[cfg(feature = "extrasimplified")]
const SIMPLIFICATION_EPSILON: Float = 0.001;

// Helpers.

/// Get the GeoJSON [`geojson::Feature`]s from the source.
#[cfg(not(target_family = "wasm"))]
pub fn get_geojson_features_from_source() -> geojson::FeatureCollection {
    let response = reqwest::blocking::get(ADDRESS).unwrap();
    let geojson_input = response.text().unwrap();

    get_geojson_features_from_string(&geojson_input)
}

// Statics.

/// The address of the GeoJSON file.
pub static ADDRESS: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_10m_time_zones.geojson";
/// The name of the timezone bincode file.
pub static TIMEZONE_BINCODE_DESTINATION_NAME: &str = "ned_time_zones.bincode";
/// The name of the cache bincode file.
pub static LOOKUP_BINCODE_DESTINATION_NAME: &str = "ned_time_zone_lookup.bincode";

// Types.

/// A representation of the [Natural Earth Data](https://www.naturalearthdata.com/)
/// [geojson](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_time_zones.geojson)
/// [`geojson::Feature`]s.
#[derive(Debug)]
#[cfg_attr(feature = "self-contained", derive(Encode))]
pub struct NedTimezone {
    /// The index of the [`NedTimezone`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The `identifier` of the [`NedTimezone`] (e.g., `America/Los_Angeles`).
    ///
    /// Essentially, it is the IANA TZ identifier.
    pub identifier: EncodableOptionString,

    /// The `description` of the [`NedTimezone`] (e.g., the countries affected).
    pub description: EncodableString,
    /// The `dst_description` of the [`NedTimezone`] (i.e., daylight savings time information).
    pub dst_description: EncodableOptionString,

    /// The `offset` of the [`NedTimezone`] (e.g., `UTC-8:00`).
    pub offset: EncodableString,

    /// The `zone_num` of the [`NedTimezone`] (e.g., `-8.0`).
    pub zone: f32,
    /// The `raw_offset` of the [`NedTimezone`] (e.g., `-28800`).
    pub raw_offset: i32,

    /// The geometry of the [`NedTimezone`].
    pub geometry: EncodableGeometry,
}

#[cfg(feature = "self-contained")]
impl Decode for NedTimezone {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let id = usize::decode(decoder)?;
        let identifier = EncodableOptionString::decode(decoder)?;
        let description = EncodableString::decode(decoder)?;
        let dst_description = EncodableOptionString::decode(decoder)?;
        let offset = EncodableString::decode(decoder)?;
        let zone = f32::decode(decoder)?;
        let raw_offset = i32::decode(decoder)?;
        let geometry = EncodableGeometry::decode(decoder)?;

        Ok(NedTimezone {
            id,
            identifier,
            description,
            dst_description,
            offset,
            zone,
            raw_offset,
            geometry,
        })
    }
}

#[cfg(feature = "self-contained")]
impl<'de> BorrowDecode<'de> for NedTimezone
where
    'de: 'static,
{
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let id = usize::decode(decoder)?;
        let identifier = EncodableOptionString::borrow_decode(decoder)?;
        let description = EncodableString::borrow_decode(decoder)?;
        let dst_description = EncodableOptionString::borrow_decode(decoder)?;
        let offset = EncodableString::borrow_decode(decoder)?;
        let zone = f32::decode(decoder)?;
        let raw_offset = i32::decode(decoder)?;
        let geometry = EncodableGeometry::borrow_decode(decoder)?;

        Ok(NedTimezone {
            id,
            identifier,
            description,
            dst_description,
            offset,
            zone,
            raw_offset,
            geometry,
        })
    }
}

impl PartialEq for NedTimezone {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<IdFeaturePair> for NedTimezone {
    fn from(value: IdFeaturePair) -> NedTimezone {
        let id = value.0;
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let dst_places = EncodableOptionString(properties.get("dst_places").unwrap().as_str().map(ToOwned::to_owned).map(Cow::Owned));
        let places = EncodableString(Cow::Owned(properties.get("places").unwrap().as_str().unwrap().to_owned()));

        let time_zone = EncodableString(Cow::Owned(properties.get("time_zone").unwrap().as_str().unwrap().to_owned()));
        let tz_name1st = EncodableOptionString(properties.get("tz_name1st").unwrap().as_str().map(ToOwned::to_owned).map(Cow::Owned));
        let zone = properties.get("zone").unwrap().as_f64().unwrap() as f32;

        let geometry: Geometry<Float> = geometry.value.clone().try_into().unwrap();
        let geometry = EncodableGeometry(simplify_geometry(geometry, SIMPLIFICATION_EPSILON));

        let raw_offset = (zone * 3600.0).round() as i32;

        NedTimezone {
            id,
            dst_description: dst_places,
            description: places,
            offset: time_zone,
            identifier: tz_name1st,
            zone,
            raw_offset,
            geometry,
        }
    }
}

impl IsTimezone for NedTimezone {
    fn identifier(&self) -> &str {
        self.identifier.as_deref().unwrap_or("")
    }
}

impl HasGeometry for NedTimezone {
    fn id(&self) -> usize {
        self.id
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry.0
    }
}

impl HasProperties for NedTimezone {
    fn properties(&self) -> Map<String, Value> {
        let mut properties = Map::new();

        properties.insert("dst_description".to_string(), Value::String(self.dst_description.as_deref().unwrap_or("").to_string()));
        properties.insert("description".to_string(), Value::String(self.description.to_string()));
        properties.insert("offset".to_string(), Value::String(self.offset.to_string()));
        properties.insert("zone".to_string(), Value::Number(serde_json::Number::from_f64(self.zone as f64).unwrap()));
        properties.insert("raw_offset".to_string(), Value::Number(serde_json::Number::from_f64(self.raw_offset as f64).unwrap()));

        properties
    }
}

#[cfg(not(target_family = "wasm"))]
impl CanGetGeoJsonFeaturesFromSource for NedTimezone {
    fn get_geojson_features_from_source() -> geojson::FeatureCollection {
        get_geojson_features_from_source()
    }
}
