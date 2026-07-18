//! Shared functionality for geo operations.

use core::str;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Formatter},
    ops::Deref,
};

use chashmap::CHashMap;
use geo::{Coord, Geometry, Intersects, Rect, SimplifyVw};
// These types are named only in the `self-contained` codec helpers; `simplify_geometry` uses them
// via `Geometry::` variants, which don't need the imports. Gating them keeps the default build warning-free.
#[cfg(feature = "self-contained")]
use geo::{LineString, MultiPolygon, Polygon};
use geojson::{Feature, FeatureCollection, GeoJson};
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
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
    T: From<IdFeaturePair> + Send,
{
    fn from(value: geojson::FeatureCollection) -> ConcreteVec<T> {
        let values = value.features.into_par_iter().enumerate().map(T::from).collect::<Vec<T>>();

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
impl<Context> Decode<Context> for EncodableString {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let data = Vec::decode(decoder)?;

        // Now, we can limit the slice to trim the null padding.
        unpad_string_alignment(&data).map(ToString::to_string).map(Cow::Owned).map(EncodableString)
    }
}

#[cfg(feature = "self-contained")]
impl<'de, Context> BorrowDecode<'de, Context> for EncodableString {
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let length = usize::decode(decoder)?;
        let slice = decoder.borrow_reader().take_bytes(length * std::mem::size_of::<u8>())?;

        // SAFETY: We know this slice is built into the binary, and it has a static lifetime.
        let slice = unsafe { std::mem::transmute::<&'_ [u8], &'static [u8]>(slice) };

        // Now, we can limit the slice to trim the null padding.
        unpad_string_alignment(slice).map(Cow::Borrowed).map(EncodableString)
    }
}

/// A wrapper for `Option<Cow<'static, str>>` to make encoding and decoding easier.
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
impl<Context> Decode<Context> for EncodableOptionString {
    fn decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: Decoder,
    {
        let variant = usize::decode(decoder)?;

        let result = match variant {
            0 => EncodableOptionString(None),
            1 => {
                let es = EncodableString::decode(decoder)?;

                EncodableOptionString(Some(es.0))
            }
            _ => panic!("Unsupported variant."),
        };

        Ok(result)
    }
}

#[cfg(feature = "self-contained")]
impl<'de, Context> BorrowDecode<'de, Context> for EncodableOptionString {
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, DecodeError>
    where
        D: BorrowDecoder<'de>,
    {
        let variant = usize::decode(decoder)?;

        let result = match variant {
            0 => EncodableOptionString(None),
            1 => {
                let es = EncodableString::borrow_decode(decoder)?;

                EncodableOptionString(Some(es.0))
            }
            _ => panic!("Unsupported variant."),
        };

        Ok(result)
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
impl<L, D, T> ToGeoJsonFeatureCollection for &L
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
    let padding = alignment - (string.as_ref().len() % alignment);

    string.as_ref().as_bytes().iter().chain(std::iter::repeat_n(&0u8, padding)).copied().collect::<Vec<u8>>()
}

/// Unpads a String after decoding to remove any null padding.
#[cfg(feature = "self-contained")]
pub fn unpad_string_alignment(data: &[u8]) -> Result<&str, DecodeError> {
    let terminator = data.iter().position(|&x| x == 0).unwrap_or(data.len());
    let slice = &data[..terminator];

    let str = str::from_utf8(slice).map_err(|e| DecodeError::Utf8 { inner: e })?;

    Ok(str)
}

/// Simplifies a [`Geometry`] using the [Visvalingam-Whyatt algorithm](https://bost.ocks.org/mike/simplify/).
///
/// For geometries that cannot be simplified, the original geometry is returned.
pub fn simplify_geometry(geometry: Geometry<Float>, simplification_epsilon: Float) -> Geometry<Float> {
    #[cfg(not(feature = "unsimplified"))]
    let geometry = match geometry {
        Geometry::Polygon(polygon) => {
            let simplified = polygon.simplify_vw(simplification_epsilon);
            Geometry::Polygon(simplified)
        }
        Geometry::MultiPolygon(multi_polygon) => {
            let simplified = multi_polygon.simplify_vw(simplification_epsilon);
            Geometry::MultiPolygon(simplified)
        }
        Geometry::LineString(line_string) => {
            let simplified = line_string.simplify_vw(simplification_epsilon);
            Geometry::LineString(simplified)
        }
        Geometry::MultiLineString(multi_line_string) => {
            let simplified = multi_line_string.simplify_vw(simplification_epsilon);
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
#[cfg_attr(coverage_nightly, coverage(off))]
fn generate_lookup_bincode<T>(bincode_input: impl AsRef<Path>, bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + Decode<()> + Send + Sync + 'static,
{
    let data = std::fs::read(bincode_input).unwrap();
    let (timezones, _len): (ConcreteVec<T>, usize) = bincode::decode_from_slice(&data, get_global_bincode_config()).unwrap();

    let cache = get_lookup_from_geometries(&timezones);

    bincode::encode_into_std_write(cache, &mut std::fs::File::create(bincode_destination).unwrap(), get_global_bincode_config()).unwrap();
}

/// Get the concrete timezones from features.
pub fn get_items_from_features<T>(features: FeatureCollection) -> ConcreteVec<T>
where
    T: HasGeometry + From<IdFeaturePair> + Send,
{
    ConcreteVec::from(features)
}

/// Generate bincode representation of the timezones.
#[cfg(feature = "self-contained")]
#[cfg_attr(coverage_nightly, coverage(off))]
fn generate_item_bincode<T>(geojson_features: FeatureCollection, bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + Encode + From<IdFeaturePair> + Send + 'static,
{
    let items: ConcreteVec<T> = get_items_from_features(geojson_features);
    bincode::encode_into_std_write(items, &mut std::fs::File::create(bincode_destination).unwrap(), get_global_bincode_config()).unwrap();
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
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn generate_bincodes<T>(geojson_features: FeatureCollection, timezone_bincode_destination: impl AsRef<Path>, lookup_bincode_destination: impl AsRef<Path>)
where
    T: HasGeometry + Encode + From<IdFeaturePair> + Decode<()> + Send + Sync + 'static,
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

// When `owned-decode` is off, geometries are borrow-decoded: their coordinate `Vec`s are
// reconstructed via `Vec::from_raw_parts` directly over the embedded asset bytes (see
// `borrow_decode_poly`). Those `Vec`s do not own heap memory, so letting `geo`'s `Vec::drop` run
// would `dealloc` a pointer into `.rodata` — undefined behavior. We forget the geometry on drop so
// that never happens; the bytes live in the binary, so there is nothing to free.
//
// SAFETY / INVARIANT: this leak-on-drop is correct *only* because, with `owned-decode` off, every
// geometry originates from `borrow_decode` (static-backed). The single selector that guarantees
// this is `decode_binary_data` in `rtz/src/geo/shared.rs` — keep the two in lockstep. With
// `owned-decode` on, geometries own real allocations and must drop normally, so there is no `Drop`.
#[cfg(not(feature = "owned-decode"))]
impl Drop for EncodableGeometry {
    fn drop(&mut self) {
        let geometry = std::mem::replace(&mut self.0, Geometry::Point(geo::Point::new(0.0, 0.0)));
        std::mem::forget(geometry);
    }
}

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
impl<Context> Decode<Context> for EncodableGeometry {
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
impl<'de, Context> BorrowDecode<'de, Context> for EncodableGeometry {
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
impl<Context> Decode<Context> for EncodableIds {
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
impl<'de, Context> BorrowDecode<'de, Context> for EncodableIds {
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

#[cfg(all(test, feature = "self-contained"))]
mod codec_tests {
    use super::*;
    use crate::base::types::Float;
    use geo::{Coord, Geometry, LineString, MultiPolygon, Polygon};
    use std::borrow::Cow;

    fn roundtrip_string(s: &str) {
        let cfg = get_global_bincode_config();
        let original = EncodableString(Cow::Owned(s.to_string()));
        let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
        let (decoded, _len): (EncodableString, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded, original, "string roundtrip failed for {s:?}");
    }

    #[test]
    fn string_roundtrips_ascii_empty_nonascii_and_alignment() {
        roundtrip_string(""); // empty
        roundtrip_string("America/Los_Angeles");
        roundtrip_string("مصر"); // non-ASCII UTF-8 through pad/unpad
        roundtrip_string("abc"); // len 3 -> 1 pad byte
        roundtrip_string("abcd"); // len 4 -> full extra 4 pad bytes (alignment boundary)
    }

    #[test]
    fn option_string_roundtrips_none_and_some() {
        let cfg = get_global_bincode_config();
        for original in [EncodableOptionString(None), EncodableOptionString(Some(Cow::Owned("x".to_string())))] {
            let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
            let (decoded, _len): (EncodableOptionString, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn ids_roundtrip_empty_and_many() {
        let cfg = get_global_bincode_config();
        for v in [Vec::<Id>::new(), vec![0u32, 1, 2, 4_000_000]] {
            let original = EncodableIds(v.clone());
            let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
            let (decoded, _len): (EncodableIds, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
            assert_eq!(decoded.0, v);
        }
    }

    #[test]
    fn geometry_roundtrips_polygon_with_interior_and_multipolygon() {
        let cfg = get_global_bincode_config();
        let exterior = LineString(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 4.0, y: 0.0 },
            Coord { x: 4.0, y: 4.0 },
            Coord { x: 0.0, y: 4.0 },
            Coord { x: 0.0, y: 0.0 },
        ]);
        let interior = LineString(vec![
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 2.0, y: 1.0 },
            Coord { x: 2.0, y: 2.0 },
            Coord { x: 1.0, y: 1.0 },
        ]);
        let poly: Polygon<Float> = Polygon::new(exterior, vec![interior]);

        let original = EncodableGeometry(Geometry::Polygon(poly.clone()));
        let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
        let (decoded, _len): (EncodableGeometry, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded.0, original.0);

        let multi = EncodableGeometry(Geometry::MultiPolygon(MultiPolygon::new(vec![poly])));
        let bytes = bincode::encode_to_vec(&multi, cfg).unwrap();
        let (decoded, _len): (EncodableGeometry, usize) = bincode::decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded.0, multi.0);
    }

    #[test]
    fn string_borrow_decode_matches_owned() {
        // The borrowed decode path builds a `Cow::Borrowed` via an internal transmute to
        // 'static. Exercising it here is sound because the source buffer outlives the decoded
        // value and `Cow::Borrowed` frees nothing on drop. We deliberately do NOT borrow-decode
        // the `Vec::from_raw_parts` types (EncodableIds / EncodableGeometry) from a local buffer:
        // their decoded Vecs would free borrowed memory on drop. Those borrow paths are already
        // exercised at runtime against the embedded 'static bincodes by the `geo::*` tests.
        let cfg = get_global_bincode_config();
        let original = EncodableString(Cow::Owned("America/Los_Angeles".to_string()));
        let bytes = bincode::encode_to_vec(&original, cfg).unwrap();
        let (decoded, _len): (EncodableString, usize) = bincode::borrow_decode_from_slice(&bytes, cfg).unwrap();
        assert_eq!(decoded.as_ref(), original.as_ref());
    }
}
