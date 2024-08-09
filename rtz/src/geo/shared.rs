//! Shared functionality for geo operations in the `rtz` crate.

// Traits.

use geo::{Contains, Coord};
use rtz_core::{
    base::types::Float,
    geo::shared::{ConcreteVec, HasGeometry, HasProperties, Id, RoundDegree, RoundLngLat, ToGeoJson},
};
use std::collections::HashMap;

/// Trait that abstracts away getting the in-memory items.
pub trait HasItemData
where
    Self: Sized,
{
    /// Gets the items from the in-memory cache for the given type.
    fn get_mem_items() -> &'static ConcreteVec<Self>;
}

/// Trait that abstracts away getting the in-memory timezones / cache.
pub trait HasLookupData: HasItemData
where
    Self: Sized,
{
    /// The type to which the lookup hash table resolves.
    type Lookup: AsRef<[Id]>;

    /// Gets the lookup hash table from the in-memory cache for the given type.
    fn get_mem_lookup() -> &'static HashMap<RoundLngLat, Self::Lookup>;
}

/// Trait that allows converting a [`u16`] into the item to which the id refers (from the global list).
// pub(crate) trait MapIntoItem<T> {
//     fn map_into_item(self) -> Option<&'static T>;
// }

// impl<T> MapIntoItem<T> for Option<&u16>
// where
//     T: HasItemData,
// {
//     fn map_into_item(self) -> Option<&'static T> {
//         let value = self?;

//         let items = T::get_mem_items();

//         items.get(*value as usize)
//     }
// }

/// Trait that allows converting a [`u16`] into the items to which the ids refer (from the global list).
pub(crate) trait MapIntoItems<T> {
    fn map_into_items(self) -> Option<Vec<&'static T>>;
}

impl<A, T> MapIntoItems<T> for Option<A>
where
    A: AsRef<[Id]>,
    T: HasItemData,
{
    fn map_into_items(self) -> Option<Vec<&'static T>> {
        let value = self?;

        let source = value.as_ref();
        let items = T::get_mem_items();

        let mut result = Vec::with_capacity(source.len());
        for id in source {
            let item = &items[*id as usize];
            result.push(item);
        }

        Some(result)
    }
}

/// Perform a decode of binary data.
#[cfg(feature = "self-contained")]
pub fn decode_binary_data<T>(data: &'static [u8]) -> T
where
    T: bincode::Decode + bincode::BorrowDecode<'static>,
{
    #[cfg(not(feature = "owned-decode"))]
    let (value, _len): (T, usize) = bincode::borrow_decode_from_slice(data, rtz_core::geo::shared::get_global_bincode_config())
        .expect("Could not decode binary data: try rebuilding with `force-rebuild` due to a likely precision difference between the generated assets and the current build.");
    #[cfg(feature = "owned-decode")]
    let (value, _len): (T, usize) = bincode::decode_from_slice(data, rtz_core::geo::shared::get_global_bincode_config())
        .expect("Could not decode binary data: try rebuilding with `force-rebuild` due to a likely precision difference between the generated assets and the current build.");

    value
}

/// Trait that abstracts away the primary end-user functionality of geo lookups.
pub trait CanPerformGeoLookup: HasLookupData + HasGeometry + HasProperties
where
    Self: 'static,
{
    /// Get the cache-driven item for a given longitude (x) and latitude (y).
    ///
    /// Some data sources allow for multiple results, so this is a vector.
    fn lookup(xf: Float, yf: Float) -> Vec<&'static Self> {
        let x = xf.floor() as RoundDegree;
        let y = yf.floor() as RoundDegree;
        
        let Some(suggestions) = Self::get_lookup_suggestions(x, y) else {
            return Vec::new();
        };

        suggestions.into_iter().filter(|&i| i.geometry().contains(&Coord { x: xf, y: yf })).collect()
    }

    /// Get the exact item for a given longitude (x) and latitude (y).
    #[allow(dead_code)]
    fn lookup_slow(xf: Float, yf: Float) -> Vec<&'static Self> {
        Self::get_mem_items().into_iter().filter(|&i| i.geometry().contains(&Coord { x: xf, y: yf })).collect()
    }

    /// Gets the geojson representation of the memory cache.
    fn memory_data_to_geojson() -> String {
        let geojson = Self::get_mem_items().to_geojson();
        geojson.to_json_value().to_string()
    }

    /// Get value from the static memory cache.
    fn get_lookup_suggestions(x: RoundDegree, y: RoundDegree) -> Option<Vec<&'static Self>> {
        let cache = Self::get_mem_lookup();
        cache.get(&(x, y)).map_into_items()
    }
}
