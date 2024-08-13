//! Shared functionality for geo operations.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Formatter},
    ops::Deref,
};

use chashmap::CHashMap;
use geo::{Coord, Geometry, Intersects, LineString, MultiPolygon, Polygon, Rect, SimplifyVw};
use geojson::{Feature, FeatureCollection, GeoJson};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde_json::{Map, Value};
use std::path::Path;

#[cfg(feature = "self-contained")]
use bincode::{
    config::Configuration,
    de::{read::BorrowReader, BorrowDecoder, Decoder},
    enc::Encoder,
    error::{DecodeError, EncodeError},
    BorrowDecode, Decode, Encode,
};

use crate::base::types::Float;

// Types.

/// An index into the global static cache.
pub type Id = u32;
/// A rounded integer.
pub type RoundDegree = i16;
/// A rounded longitude and latitude.
pub type RoundLngLat = (RoundDegree, RoundDegree);
/// An `(id, Feature)` pair.
pub type IdFeaturePair = (usize, geojson::Feature);

// Concrete helpers.

/// A concrete collection of concrete values.
#[derive(Debug)]
#[cfg_attr(feature = "self-contained", derive(Encode, Decode))]
pub struct ConcreteVec<T>(Vec<T>)
where
    T: 'static;

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

// Cow helpers.

/// A wrapper for `Cow<'static, str>` to make encoding and decoding easier.
#[cfg(feature = "self-contained")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EncodableString(pub Cow<'static, str>);

impl AsRef<str> for EncodableString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Deref for EncodableString {
    type Target = Cow<'static, str>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for EncodableString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "self-contained")]
impl Encode for EncodableString {
    fn encode<E>(&self, encoder: &mut E) -> Result<(), EncodeError>
    where
        E: Encoder,
    {
        let data = pad_string_alignment(self);

        data.encode(encoder)
    }
}

#[cfg(feature = "self-contained")]
impl Decode for EncodableString {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let cow = Cow::<'static, str>::decode(decoder)?;

        Ok(EncodableString(cow))
    }
}

#[cfg(feature = "self-contained")]
impl<'de> BorrowDecode<'de> for EncodableString {
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let cow = Cow::<'static, str>::decode(decoder)?;

        Ok(EncodableString(cow))
    }
}

/// A wrapper for `Option<Cow<'static, str>>` to make encoding and decoding easier.
#[cfg(feature = "self-contained")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EncodableOptionString(pub Option<Cow<'static, str>>);

impl EncodableOptionString {
    /// Converts from `EncodableOptionString`` to `Option<&Cow<'static, str>>``.
    pub fn as_ref(&self) -> Option<&Cow<'static, str>> {
        self.0.as_ref()
    }
}

impl Deref for EncodableOptionString {
    type Target = Option<Cow<'static, str>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for EncodableOptionString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.as_ref() {
            None => write!(f, "None"),
            Some(cow) => write!(f, "{}", cow),
        }
    }
}

#[cfg(feature = "self-contained")]
impl Encode for EncodableOptionString {
    fn encode<E>(&self, encoder: &mut E) -> Result<(), EncodeError>
    where
        E: Encoder,
    {
        match self.as_ref() {
            None => 0usize.encode(encoder),
            Some(cow) => {
                // Descriminant.
                1usize.encode(encoder)?;

                // Padded data.
                let data = pad_string_alignment(cow);

                data.encode(encoder)
            }
        }
    }
}

#[cfg(feature = "self-contained")]
impl Decode for EncodableOptionString {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let variant = usize::decode(decoder)?;

        let cow = match variant {
            0 => None,
            1 => {
                let cow = Cow::<'static, str>::decode(decoder)?;

                Some(cow)
            }
            _ => panic!("Unsupported variant."),
        };

        Ok(EncodableOptionString(cow))
    }
}

#[cfg(feature = "self-contained")]
impl<'de> BorrowDecode<'de> for EncodableOptionString {
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let variant = usize::decode(decoder)?;

        let cow = match variant {
            0 => None,
            1 => {
                let cow = Cow::<'static, str>::decode(decoder)?;

                Some(cow)
            }
            _ => panic!("Unsupported variant."),
        };

        Ok(EncodableOptionString(cow))
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

/// Pads a String before encoding to ensure that the string is aligned to the correct byte boundary.
#[cfg(feature = "self-contained")]
pub fn pad_string_alignment(string: impl AsRef<str>) -> Vec<u8> {
    let alignment = std::mem::align_of::<Float>();
    let padding = alignment - (string.as_ref().as_bytes().len() % alignment);

    string.as_ref().as_bytes().iter().chain(std::iter::repeat(&0u8).take(padding)).copied().collect::<Vec<u8>>()
}

/// Simplifies a [`Geometry`] using the [Visvalingam-Whyatt algorithm](https://bost.ocks.org/mike/simplify/).
///
/// For geometries that cannot be simplified, the original geometry is returned.
pub fn simplify_geometry(geometry: Geometry<Float>, simplification_epsilon: Float) -> Geometry<Float> {
    #[cfg(not(feature = "unsimplified"))]
    let geometry = match geometry {
        Geometry::Polygon(polygon) => {
            let simplified = polygon.simplify_vw(&simplification_epsilon);
            Geometry::Polygon(simplified)
        }
        Geometry::MultiPolygon(multi_polygon) => {
            let simplified = multi_polygon.simplify_vw(&simplification_epsilon);
            Geometry::MultiPolygon(simplified)
        }
        Geometry::LineString(line_string) => {
            let simplified = line_string.simplify_vw(&simplification_epsilon);
            Geometry::LineString(simplified)
        }
        Geometry::MultiLineString(multi_line_string) => {
            let simplified = multi_line_string.simplify_vw(&simplification_epsilon);
            Geometry::MultiLineString(simplified)
        }
        g => g,
    };

    geometry
}

/// Get the cache from the timezones.
pub fn get_lookup_from_geometries<T>(geometries: &ConcreteVec<T>) -> HashMap<RoundLngLat, EncodableIds>
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
                    intersected.push(g.id() as Id);
                }
            }

            map.insert((x as RoundDegree, y as RoundDegree), intersected);
        }
    });

    let mut cache = HashMap::new();
    for (key, value) in map.into_iter() {
        cache.insert(key, EncodableIds(value));
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
    T: HasGeometry + Decode + Send + Sync + 'static,
{
    let data = std::fs::read(bincode_input).unwrap();
    let (timezones, _len): (ConcreteVec<T>, usize) = bincode::decode_from_slice(&data, get_global_bincode_config()).unwrap();

    let cache = get_lookup_from_geometries(&timezones);

    std::fs::write(bincode_destination, bincode::encode_to_vec(cache, get_global_bincode_config()).unwrap()).unwrap();
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
    T: HasGeometry + Encode + From<IdFeaturePair> + 'static,
{
    let items: ConcreteVec<T> = get_items_from_features(geojson_features);

    std::fs::write(bincode_destination, bincode::encode_to_vec(items, get_global_bincode_config()).unwrap()).unwrap();
}

/// Get the GeoJSON features from the binary assets.
pub fn get_geojson_features_from_file(geojson_input: impl AsRef<Path>) -> FeatureCollection {
    let geojson = std::fs::read_to_string(geojson_input).unwrap();
    FeatureCollection::try_from(geojson.parse::<GeoJson>().unwrap()).unwrap()
}

/// Get the GeoJSON features from the binary assets.
pub fn get_geojson_features_from_string(geojson_input: &str) -> FeatureCollection {
    FeatureCollection::try_from(geojson_input.parse::<GeoJson>().unwrap()).unwrap()
}

/// Get the GeoJSON feature from the binary assets.
pub fn get_geojson_feature_from_file(geojson_input: impl AsRef<Path>) -> Feature {
    let geojson = std::fs::read_to_string(geojson_input).unwrap();
    Feature::try_from(geojson.parse::<GeoJson>().unwrap()).unwrap()
}

/// Get the GeoJSON feature from the binary assets.
pub fn get_geojson_feature_from_string(geojson_input: &str) -> Feature {
    Feature::try_from(geojson_input.parse::<GeoJson>().unwrap()).unwrap()
}

/// Generates new bincodes for the timezones and the cache from the GeoJSON.
#[cfg(feature = "self-contained")]
pub fn generate_bincodes<T>(geojson_features: FeatureCollection, timezone_bincode_destination: impl AsRef<Path>, lookup_bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + Encode + From<IdFeaturePair> + Decode + Send + Sync + 'static,
{
    generate_item_bincode::<T>(geojson_features, timezone_bincode_destination.as_ref());
    generate_lookup_bincode::<T>(timezone_bincode_destination, lookup_bincode_destination);
}

// Helpers to get GeoJSON features from a source.

/// Trait that supports getting the GeoJSON features from a source.
pub trait CanGetGeoJsonFeaturesFromSource {
    /// Get the GeoJSON features from a source.
    fn get_geojson_features_from_source() -> geojson::FeatureCollection;
}

// Bincode helpers.

/// Computes the best bincode to be used for the target architecture.
#[cfg(all(feature = "self-contained", target_endian = "big"))]

pub fn get_global_bincode_config() -> Configuration<bincode::config::BigEndian, bincode::config::Fixint> {
    bincode::config::legacy().with_big_endian()
}

/// Computes the best bincode to be used for the target architecture.
#[cfg(all(feature = "self-contained", target_endian = "little"))]
pub fn get_global_bincode_config() -> Configuration<bincode::config::LittleEndian, bincode::config::Fixint> {
    bincode::config::legacy()
}

// Special encoding / decoding logic for geometries.

/// A wrapped [`Geometry`] that can be encoded and decoded via bincode.
#[derive(Debug)]
pub struct EncodableGeometry(pub Geometry<Float>);

#[cfg(feature = "self-contained")]
fn encode_poly<E>(polygon: &Polygon<Float>, encoder: &mut E) -> Result<(), EncodeError>
where
    E: Encoder,
{
    let exterior = &polygon.exterior().0;

    // Encode the exterior length.
    exterior.len().encode(encoder)?;

    // Encode the exterior points.
    for point in exterior {
        point.x.encode(encoder)?;
        point.y.encode(encoder)?;
    }

    let interiors = polygon.interiors();

    // Encode the number of interiors.
    interiors.len().encode(encoder)?;

    // Encode the interiors.
    for interior in interiors {
        let interior = &interior.0;

        // Encode the interior length.
        interior.len().encode(encoder)?;

        // Encode the interior points.
        for point in interior {
            point.x.encode(encoder)?;
            point.y.encode(encoder)?;
        }
    }

    Ok(())
}

#[cfg(feature = "self-contained")]
impl Encode for EncodableGeometry {
    fn encode<E>(&self, encoder: &mut E) -> Result<(), EncodeError>
    where
        E: Encoder,
    {
        match &self.0 {
            Geometry::Polygon(polygon) => {
                // Encode the variant.
                0usize.encode(encoder)?;

                encode_poly(polygon, encoder)?;
            }
            Geometry::MultiPolygon(multi_polygon) => {
                // Encode the variant.
                1usize.encode(encoder)?;

                let polygons = &multi_polygon.0;

                // Encode the number of polygons.
                polygons.len().encode(encoder)?;

                // Encode the polygons.
                for polygon in polygons {
                    encode_poly(polygon, encoder)?;
                }
            }
            _ => panic!("Unsupported geometry variant."),
        }

        Ok(())
    }
}

#[cfg(feature = "self-contained")]
fn decode_poly<D>(decoder: &mut D) -> Result<Polygon<Float>, DecodeError>
where
    D: Decoder,
{
    let exterior_len = usize::decode(decoder)?;

    let mut exterior = Vec::with_capacity(exterior_len);

    for _ in 0..exterior_len {
        let x = Float::decode(decoder)?;
        let y = Float::decode(decoder)?;

        exterior.push(Coord { x, y });
    }

    let interior_len = usize::decode(decoder)?;

    let mut interiors = Vec::with_capacity(interior_len);

    for _ in 0..interior_len {
        let interior_len = usize::decode(decoder)?;

        let mut interior = Vec::with_capacity(interior_len);

        for _ in 0..interior_len {
            let x = Float::decode(decoder)?;
            let y = Float::decode(decoder)?;

            interior.push(Coord { x, y });
        }

        interiors.push(LineString(interior));
    }

    Ok(Polygon::new(LineString(exterior), interiors))
}

#[cfg(feature = "self-contained")]
fn borrow_decode_poly<'de, D>(decoder: &mut D) -> Result<Polygon<Float>, DecodeError>
where
    D: BorrowDecoder<'de>,
{
    let exterior_len = usize::decode(decoder)?;
    let exterior_slice = decoder.borrow_reader().take_bytes(exterior_len * std::mem::size_of::<Float>() * 2)?;

    // SAFETY: Perform unholy rites, and summon the devil, lol.
    // Basically, this is an extreme optimization to prevent loading huge amounts of data into memory that are already
    // in memory as part of the binary assets.
    let exterior = unsafe { Vec::from_raw_parts(exterior_slice.as_ptr() as *mut Coord<Float>, exterior_len, exterior_len) };

    let interior_len = usize::decode(decoder)?;

    let mut interiors = Vec::with_capacity(interior_len);

    for _ in 0..interior_len {
        let interior_len = usize::decode(decoder)?;
        let interior_slice = decoder.borrow_reader().take_bytes(interior_len * std::mem::size_of::<Float>() * 2)?;

        // SAFETY: Perform unholy rites again: see above.
        let interior = unsafe { Vec::from_raw_parts(interior_slice.as_ptr() as *mut Coord<Float>, interior_len, interior_len) };

        interiors.push(LineString(interior));
    }

    Ok(Polygon::new(LineString(exterior), interiors))
}

#[cfg(feature = "self-contained")]
impl Decode for EncodableGeometry {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let variant = usize::decode(decoder)?;

        let geometry = match variant {
            0 => {
                let polygon = decode_poly(decoder)?;

                Geometry::Polygon(polygon)
            }
            1 => {
                let polygon_len = usize::decode(decoder)?;

                let mut polygons = Vec::with_capacity(polygon_len);

                for _ in 0..polygon_len {
                    let polygon = decode_poly(decoder)?;

                    polygons.push(polygon);
                }

                Geometry::MultiPolygon(MultiPolygon::new(polygons))
            }
            _ => panic!("Unsupported geometry variant."),
        };

        Ok(EncodableGeometry(geometry))
    }
}

#[cfg(feature = "self-contained")]
impl<'de> BorrowDecode<'de> for EncodableGeometry {
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let variant = usize::decode(decoder)?;

        let geometry = match variant {
            0 => {
                let polygon = borrow_decode_poly(decoder)?;

                Geometry::Polygon(polygon)
            }
            1 => {
                let polygon_len = usize::decode(decoder)?;

                let mut polygons = Vec::with_capacity(polygon_len);

                for _ in 0..polygon_len {
                    let polygon = borrow_decode_poly(decoder)?;

                    polygons.push(polygon);
                }

                Geometry::MultiPolygon(MultiPolygon::new(polygons))
            }
            _ => panic!("Unsupported geometry variant."),
        };

        Ok(EncodableGeometry(geometry))
    }
}

/// A wrapped ['Vec`] that can be encoded and decoded via bincode.
#[derive(Debug)]
pub struct EncodableIds(pub Vec<Id>);

impl Deref for EncodableIds {
    type Target = Vec<Id>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[Id]> for EncodableIds {
    fn as_ref(&self) -> &[Id] {
        &self.0
    }
}

#[cfg(feature = "self-contained")]
impl Encode for EncodableIds {
    fn encode<E>(&self, encoder: &mut E) -> Result<(), EncodeError>
    where
        E: Encoder,
    {
        // Encode the exterior length.
        self.0.len().encode(encoder)?;

        // Encode the exterior points.
        for x in &self.0 {
            x.encode(encoder)?;
        }

        Ok(())
    }
}

#[cfg(feature = "self-contained")]
impl Decode for EncodableIds {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let len = usize::decode(decoder)?;

        let mut vec = Vec::with_capacity(len);

        for _ in 0..len {
            let x = Id::decode(decoder)?;

            vec.push(x);
        }

        Ok(EncodableIds(vec))
    }
}

#[cfg(feature = "self-contained")]
impl<'de> BorrowDecode<'de> for EncodableIds {
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let len = usize::decode(decoder)?;
        let slice = decoder.borrow_reader().take_bytes(len * std::mem::size_of::<Id>())?;

        // SAFETY: Perform unholy rites again: see above.
        let vec = unsafe { Vec::from_raw_parts(slice.as_ptr() as *mut Id, len, len) };

        Ok(EncodableIds(vec))
    }
}
