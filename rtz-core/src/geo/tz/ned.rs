//! All of the geo-specific functions for NED TZ lookups.

use std::{collections::HashMap, path::Path, ops::Deref};

use chashmap::CHashMap;
use geo::{Coord, Intersects, Rect, Geometry};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::{Serialize, Deserialize};

use geojson::{FeatureCollection, GeoJson};

/// Get the cache from the timezones.
pub fn get_cache_from_timezones(timezones: &ConcreteTimezones) -> HashMap<RoundLngLat, Vec<i16>> {
    let map = CHashMap::new();

    (-180..180).into_par_iter().for_each(|x| {
        for y in -90..90 {
            let xf = x as f64;
            let yf = y as f64;

            let rect = Rect::new(Coord { x: xf, y: yf }, Coord { x: xf + 1.0, y: yf + 1.0 });

            let mut intersected = Vec::new();

            for tz in timezones {
                if tz.geometry.intersects(&rect) {
                    intersected.push(tz.id as RoundInt);
                }
            }

            map.insert((x as RoundInt, y as RoundInt), intersected);
        }
    });

    let mut cache = HashMap::new();
    for (key, value) in map.into_iter() {
        cache.insert(key, value);
    }

    cache
}

/// Generate the bincode representation of the 100km cache.
///
/// "100km" is a bit of a misnomer.  This is really 100km _at the equator_, but this
/// makes it easier to reason about what the caches are doing.
#[cfg(feature = "self-contained")]
fn generate_cache_bincode(bincode_input: impl AsRef<Path>, bincode_destination: impl AsRef<Path>) {
    let data = std::fs::read(bincode_input).unwrap();
    let (timezones, _len): (ConcreteTimezones, usize) = bincode::serde::decode_from_slice(&data, bincode::config::standard()).unwrap();
    
    let cache = get_cache_from_timezones(&timezones);

    std::fs::write(bincode_destination, bincode::serde::encode_to_vec(cache, bincode::config::standard()).unwrap()).unwrap();
}

/// Get the concrete timezones from features.
pub fn get_timezones_from_features(features: FeatureCollection) -> ConcreteTimezones {
    ConcreteTimezones::from(&features)
}

/// Generate bincode representation of the timezones.
#[cfg(feature = "self-contained")]
fn generate_timezone_bincode(geojson_features: FeatureCollection, bincode_destination: impl AsRef<Path>) {
    let timezones = get_timezones_from_features(geojson_features);

    std::fs::write(bincode_destination, bincode::serde::encode_to_vec(timezones, bincode::config::standard()).unwrap()).unwrap();
}

/// Generates new bincodes for the timezones and the cache from the GeoJSON.
#[cfg(feature = "self-contained")]
pub fn generate_bincodes(geojson_features: FeatureCollection, timezone_bincode_destination: impl AsRef<Path>, cache_bincode_destination: impl AsRef<Path>) {
    generate_timezone_bincode(geojson_features, timezone_bincode_destination.as_ref());
    generate_cache_bincode(timezone_bincode_destination, cache_bincode_destination);
}

/// Get the GeoJSON features from the binary assets.
pub fn get_geojson_features_from_file(geojson_input: impl AsRef<Path>) -> FeatureCollection {
    let tz_geojson = std::fs::read_to_string(geojson_input).unwrap();
    FeatureCollection::try_from(tz_geojson.parse::<GeoJson>().unwrap()).unwrap()
}

/// Get the GeoJSON features from the binary assets.
pub fn get_geojson_features_from_string(geojson_input: &str) -> FeatureCollection {
    FeatureCollection::try_from(geojson_input.parse::<GeoJson>().unwrap()).unwrap()
}

// Statics.

pub static GEOJSON_ADDRESS: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_10m_time_zones.geojson";

// Types.

// Helper types.

pub type RoundInt = i16;
pub type RoundLngLat = (RoundInt, RoundInt);
//pub type LngLat = (f64, f64);

/// A collection of `id`s into the global time zone static cache.
pub type TimezoneIds = [RoundInt; 10];
/// A [`Timezone`] static reference.
pub type TimezoneRef = &'static Timezone;
/// A collection of [`Timezone`] static references.
pub type TimezoneRefs = Vec<TimezoneRef>;

// Geo Types.

/// A concrete collection of [`Timezone`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConcreteTimezones(Vec<Timezone>);

impl Deref for ConcreteTimezones {
    type Target = Vec<Timezone>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&geojson::FeatureCollection> for ConcreteTimezones {
    fn from(value: &geojson::FeatureCollection) -> ConcreteTimezones {
        ConcreteTimezones(value.features.iter().enumerate().map(Timezone::from).collect())
    }
}

impl IntoIterator for ConcreteTimezones {
    type IntoIter = std::vec::IntoIter<Timezone>;
    type Item = Timezone;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConcreteTimezones {
    type IntoIter = std::slice::Iter<'a, Timezone>;
    type Item = &'a Timezone;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// A representation of the [Natural Earth Data](https://www.naturalearthdata.com/)
/// [geojson](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_time_zones.geojson)
/// [`geojson::Feature`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct Timezone {
    /// The index of the [`Timezone`] in the global static cache.
    pub id: usize,
    /// The `objectid` of the [`Timezone`].
    pub objectid: u64,
    /// The `friendly_name` of the [`Timezone`] (e.g., `America/Los_Angeles`).
    /// 
    /// Essentially, it is the IANA TZ identifier.
    pub friendly_name: Option<String>,

    /// The `description` of the [`Timezone`] (e.g., the countries affected).
    pub description: String,
    /// The `dst_description` of the [`Timezone`] (i.e., daylight savings time information).
    pub dst_description: Option<String>,

    /// The `offset_str` of the [`Timezone`] (e.g., `UTC-8:00`).
    pub offset_str: String,

    /// The `zone_num` of the [`Timezone`] (e.g., `-8`).
    pub zone_num: f64,
    /// The `zone_str` of the [`Timezone`] (e.g., `"-9.5"`).
    pub zone_str: String,
    /// The `raw_offset` of the [`Timezone`] (e.g., `-28800`).
    pub raw_offset: i64,

    /// The bounding box of the [`Timezone`].
    pub bbox: Rect,
    /// The geometry of the [`Timezone`].
    pub geometry: Geometry,
}

impl PartialEq for Timezone {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<(usize, &geojson::Feature)> for Timezone {
    fn from(value: (usize, &geojson::Feature)) -> Timezone {
        let id = value.0;
        let bbox = value.1.bbox.as_ref().unwrap();
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let objectid = properties.get("objectid").unwrap().as_u64().unwrap();
        let dst_places = properties.get("dst_places").unwrap().as_str().map(ToOwned::to_owned);
        let name = properties.get("name").unwrap().as_str().unwrap().to_owned();
        let places = properties.get("places").unwrap().as_str().unwrap().to_owned();

        let time_zone = properties.get("time_zone").unwrap().as_str().unwrap().to_owned();
        let tz_name1st = properties.get("tz_name1st").unwrap().as_str().map(ToOwned::to_owned);
        let zone = properties.get("zone").unwrap().as_f64().unwrap();

        let bbox = Rect::new(Coord { x: bbox[0], y: bbox[1] }, Coord { x: bbox[2], y: bbox[3] });

        let geometry: Geometry = geometry.value.clone().try_into().unwrap();

        //let mut parsable_offset = time_zone.clone();
        //parsable_offset.remove_matches('+');
        let raw_offset = (name.parse::<f64>().unwrap() * 3600.0).round() as i64;

        Timezone {
            id,
            objectid,
            dst_description: dst_places,
            zone_str: name,
            description: places,
            offset_str: time_zone,
            friendly_name: tz_name1st,
            zone_num: zone,
            raw_offset,
            bbox,
            geometry,
        }
    }
}

// Helper methods.

pub fn i16_vec_to_tomezoneids(value: Vec<i16>) -> TimezoneIds {
    if value.len() > 10 {
        panic!("Cannot convert a Vec<i16> with more than 10 elements into a TimezoneIds.");
    }

    [
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
    ]
}