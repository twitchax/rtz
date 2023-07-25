//! The `shared` module.  Contains types and helpers pertinent to all TZ implementations.

// Types.

use std::{ops::Deref, collections::HashMap, path::Path};

use chashmap::CHashMap;
use geo::{Geometry, Rect, Coord, Intersects};
use geojson::{FeatureCollection, GeoJson};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::{{Serialize, Deserialize}, de::DeserializeOwned};

use crate::base::types::Float;

/// A rounded integer.
pub type RoundInt = i16;
/// A rounded longitude and latitude.
pub type RoundLngLat = (RoundInt, RoundInt);
//pub type LngLat = (f64, f64);
/// An `(id, Feature)` pair.
pub type IdFeaturePair = (usize, geojson::Feature);

/// This number is selected based on the existing data, and may need to be increased
/// across dataset versions.  However, it is helpful to keep this as an array
/// for cache locality in the map.
const TIMEZONE_LIST_LENGTH: usize = 7;

/// A collection of `id`s into the global time zone static cache.
pub type TimezoneIds = [RoundInt; TIMEZONE_LIST_LENGTH];

// Traits.

/// A trait for types that are a timezone and have a [`Geometry`].
/// 
/// Helps abstract away this property so the helper methods can be generalized.
pub trait IsTimezone {
    /// Get the `id` of the [`IsTimezone`].
    fn id(&self) -> usize;
    /// Get the `identifier` of the [`IsTimezone`].
    fn identifier(&self) -> &str;
    /// Get the [`Geometry`] of the [`IsTimezone`].
    fn geometry(&self) -> &Geometry<Float>;
}

// Concrete helpers.

/// A concrete collection of [`Timezone`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConcreteVec<T>(Vec<T>);

impl<T> Deref for ConcreteVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<geojson::FeatureCollection> for ConcreteVec<T>
where
    T: From<IdFeaturePair>,
{
    fn from(value: geojson::FeatureCollection) -> ConcreteVec<T> {
        let values = value.features.into_iter().enumerate().map(T::from).collect::<Vec<T>>();

        ConcreteVec(values)
    }
}

impl<T> IntoIterator for ConcreteVec<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ConcreteVec<T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// Helper methods.

/// Convert a [`Vec`] of [`i16`]s into [`NedTimezoneIds`].
pub fn i16_vec_to_tomezoneids(value: Vec<i16>) -> TimezoneIds {
    if value.len() > TIMEZONE_LIST_LENGTH {
        panic!("Cannot convert a Vec<i16> with more than `TIMEZONE_LIST_LENGTH` elements into a TimezoneIds.");
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
    ]
}

// Shared helper methods.

/// Get the cache from the timezones.
pub fn get_cache_from_timezones<T>(timezones: &ConcreteVec<T>) -> HashMap<RoundLngLat, Vec<i16>>
where
    T: IsTimezone + Send + Sync,
{
    let map = CHashMap::new();

    (-180..180).into_par_iter().for_each(|x| {
        for y in -90..90 {
            let xf = x as Float;
            let yf = y as Float;

            let rect = Rect::new(Coord { x: xf, y: yf }, Coord { x: xf + 1.0, y: yf + 1.0 });

            let mut intersected = Vec::new();

            for tz in timezones {
                if tz.geometry().intersects(&rect) {
                    intersected.push(tz.id() as RoundInt);
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
fn generate_cache_bincode<T>(bincode_input: impl AsRef<Path>, bincode_destination: impl AsRef<Path>)
where
    T: IsTimezone + DeserializeOwned + Send + Sync,
{
    let data = std::fs::read(bincode_input).unwrap();
    let (timezones, _len): (ConcreteVec<T>, usize) = bincode::serde::decode_from_slice(&data, bincode::config::standard()).unwrap();

    let cache = get_cache_from_timezones(&timezones);

    std::fs::write(bincode_destination, bincode::serde::encode_to_vec(cache, bincode::config::standard()).unwrap()).unwrap();
}

/// Get the concrete timezones from features.
pub fn get_timezones_from_features<T>(features: FeatureCollection) -> ConcreteVec<T>
where
    T: IsTimezone + From<IdFeaturePair>,
{
    ConcreteVec::from(features)
}

/// Generate bincode representation of the timezones.
#[cfg(feature = "self-contained")]
fn generate_timezone_bincode<T>(geojson_features: FeatureCollection, bincode_destination: impl AsRef<Path>)
where
    T: IsTimezone + Serialize + From<IdFeaturePair>,
{
    let timezones: ConcreteVec<T> = get_timezones_from_features(geojson_features);

    std::fs::write(bincode_destination, bincode::serde::encode_to_vec(timezones, bincode::config::standard()).unwrap()).unwrap();
}

/// Generates new bincodes for the timezones and the cache from the GeoJSON.
#[cfg(feature = "self-contained")]
pub fn generate_bincodes<T>(geojson_features: FeatureCollection, timezone_bincode_destination: impl AsRef<Path>, cache_bincode_destination: impl AsRef<Path>)
where
    T: IsTimezone + Serialize + From<IdFeaturePair> + DeserializeOwned + Send + Sync,
{
    generate_timezone_bincode::<T>(geojson_features, timezone_bincode_destination.as_ref());
    generate_cache_bincode::<T>(timezone_bincode_destination, cache_bincode_destination);
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