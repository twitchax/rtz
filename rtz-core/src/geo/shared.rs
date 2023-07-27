//! Shared functionality for geo operations.

// Statics.

use std::ops::Deref;

use geo::{Geometry, SimplifyVw};
use geojson::GeoJson;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    /// Get the [`Geometry`] of the [`IsTimezone`].
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
