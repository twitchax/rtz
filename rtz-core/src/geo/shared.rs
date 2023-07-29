//! Shared functionality for geo operations.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use std::{collections::HashMap, ops::Deref};

use chashmap::CHashMap;
use geo::{Coord, Geometry, Intersects, Rect, SimplifyVw};
use geojson::{Feature, FeatureCollection, GeoJson};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;

use crate::base::types::Float;

// Constants.

#[cfg(not(feature = "extrasimplified"))]
const SIMPLIFICATION_EPSILON: Float = 0.0001;
#[cfg(feature = "extrasimplified")]
const SIMPLIFICATION_EPSILON: Float = 0.01;

// Types.

/// A rounded integer.
pub type RoundInt = i16;
/// A rounded longitude and latitude.
pub type RoundLngLat = (RoundInt, RoundInt);
//pub type LngLat = (f64, f64);
/// An `(id, Feature)` pair.
pub type IdFeaturePair = (usize, geojson::Feature);

// Concrete helpers.

/// A concrete collection of concrete values.
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

// Traits.

/// A trait for types that have a [`Geometry`].
///
/// Helps abstract away this property so the helper methods can be generalized.
pub trait HasGeometry {
    /// Get the `id` of the [`HasGeometry`].
    fn id(&self) -> usize;
    /// Get the [`Geometry`] of the [`HasGeometry`].
    fn geometry(&self) -> &Geometry<Float>;
}

/// A trait for types that have properties.
pub trait HasProperties {
    /// Get the properties of the [`HasProperties`].
    fn properties(&self) -> Map<String, Value>;
}

/// A trait that allows types to be converted to GeoJSON.
pub trait ToGeoJsonFeature {
    /// Convert the type to GeoJSON.
    fn to_feature(&self) -> geojson::Feature;
}

impl<T> ToGeoJsonFeature for T
where
    T: HasGeometry + HasProperties,
{
    fn to_feature(&self) -> geojson::Feature {
        let geometry = self.geometry();
        let properties = self.properties();

        geojson::Feature {
            properties: Some(properties),
            geometry: Some(geojson::Geometry::from(geometry)),
            ..geojson::Feature::default()
        }
    }
}

/// A trait that allows for iterator types to be converted to GeoJSON.
pub trait ToGeoJsonFeatureCollection {
    /// Convert the type to GeoJSON.
    fn to_feature_collection(&self) -> geojson::FeatureCollection;
}

/// Implementation specifically for [`ConcreteVec`].
impl<'a, L, D, T> ToGeoJsonFeatureCollection for &'a L
where
    L: Deref<Target = D>,
    D: Deref<Target = [T]>,
    T: ToGeoJsonFeature + 'static,
{
    fn to_feature_collection(&self) -> geojson::FeatureCollection {
        let features = self.iter().map(|x| x.to_feature()).collect();

        geojson::FeatureCollection {
            features,
            bbox: None,
            foreign_members: None,
        }
    }
}

/// A trait to convert to GeoJSON.
pub trait ToGeoJson {
    /// Convert the type to GeoJSON.
    fn to_geojson(&self) -> GeoJson;
}

impl<T> ToGeoJson for T
where
    T: ToGeoJsonFeatureCollection,
{
    fn to_geojson(&self) -> GeoJson {
        GeoJson::FeatureCollection(self.to_feature_collection())
    }
}

// Helper methods.

/// Simplifies a [`Geometry`] using the [Visvalingam-Whyatt algorithm](https://bost.ocks.org/mike/simplify/).
///
/// For geometries that cannot be simplified, the original geometry is returned.
pub fn simplify_geometry(geometry: Geometry<Float>) -> Geometry<Float> {
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
        Geometry::LineString(line_string) => {
            let simplified = line_string.simplify_vw(&SIMPLIFICATION_EPSILON);
            Geometry::LineString(simplified)
        }
        Geometry::MultiLineString(multi_line_string) => {
            let simplified = multi_line_string.simplify_vw(&SIMPLIFICATION_EPSILON);
            Geometry::MultiLineString(simplified)
        }
        g => g,
    };

    geometry
}

/// Get the cache from the timezones.
pub fn get_lookup_from_geometries<T>(geometries: &ConcreteVec<T>) -> HashMap<RoundLngLat, Vec<i16>>
where
    T: HasGeometry + Send + Sync,
{
    let map = CHashMap::new();

    (-180..180).into_par_iter().for_each(|x| {
        for y in -90..90 {
            let xf = x as Float;
            let yf = y as Float;

            let rect = Rect::new(Coord { x: xf, y: yf }, Coord { x: xf + 1.0, y: yf + 1.0 });

            let mut intersected = Vec::new();

            for g in geometries {
                if g.geometry().intersects(&rect) {
                    intersected.push(g.id() as RoundInt);
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
fn generate_lookup_bincode<T>(bincode_input: impl AsRef<Path>, bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + DeserializeOwned + Send + Sync,
{
    let data = std::fs::read(bincode_input).unwrap();
    let (timezones, _len): (ConcreteVec<T>, usize) = bincode::serde::decode_from_slice(&data, bincode::config::standard()).unwrap();

    let cache = get_lookup_from_geometries(&timezones);

    std::fs::write(bincode_destination, bincode::serde::encode_to_vec(cache, bincode::config::standard()).unwrap()).unwrap();
}

/// Get the concrete timezones from features.
pub fn get_items_from_features<T>(features: FeatureCollection) -> ConcreteVec<T>
where
    T: HasGeometry + From<IdFeaturePair>,
{
    ConcreteVec::from(features)
}

/// Generate bincode representation of the timezones.
#[cfg(feature = "self-contained")]
fn generate_item_bincode<T>(geojson_features: FeatureCollection, bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + Serialize + From<IdFeaturePair>,
{
    let timezones: ConcreteVec<T> = get_items_from_features(geojson_features);

    std::fs::write(bincode_destination, bincode::serde::encode_to_vec(timezones, bincode::config::standard()).unwrap()).unwrap();
}

/// Get the GeoJSON features from the binary assets.
pub fn get_geojson_features_from_file(geojson_input: impl AsRef<Path>) -> FeatureCollection {
    let tz_geojson = std::fs::read_to_string(geojson_input).unwrap();
    FeatureCollection::try_from(tz_geojson.parse::<GeoJson>().unwrap()).unwrap()
}

/// Get the GeoJSON feature from a binary assets.
pub fn get_geojson_feature_from_string(geojson_input: &str) -> Feature {
    Feature::try_from(geojson_input.parse::<GeoJson>().unwrap()).unwrap()
}

/// Get the GeoJSON features a the binary assets.
pub fn get_geojson_features_from_string(geojson_input: &str) -> FeatureCollection {
    FeatureCollection::try_from(geojson_input.parse::<GeoJson>().unwrap()).unwrap()
}

/// Generates new bincodes for the timezones and the cache from the GeoJSON.
#[cfg(feature = "self-contained")]
pub fn generate_bincodes<T>(geojson_features: FeatureCollection, timezone_bincode_destination: impl AsRef<Path>, lookup_bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + Serialize + From<IdFeaturePair> + DeserializeOwned + Send + Sync,
{
    generate_item_bincode::<T>(geojson_features, timezone_bincode_destination.as_ref());
    generate_lookup_bincode::<T>(timezone_bincode_destination, lookup_bincode_destination);
}
