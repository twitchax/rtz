//! The `shared` module.  Contains types and helpers pertinent to all TZ implementations.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use crate::geo::shared::{HasGeometry, HasProperties, RoundInt};

// Types.

/// This number is selected based on the existing data, and may need to be increased
/// across dataset versions.  However, it is helpful to keep this as an array
/// for cache locality in the map.
const TIMEZONE_LOOKUP_LENGTH: usize = 7;

/// A collection of `id`s into the global time zone static cache.
pub type TimezoneIds = [RoundInt; TIMEZONE_LOOKUP_LENGTH];

// Traits.

/// A trait for types that are a timezone and have a [`Geometry`].
///
/// Helps abstract away this property so the helper methods can be generalized.
pub trait IsTimezone: HasGeometry + HasProperties {
    /// Get the `identifier` of the [`IsTimezone`].
    fn identifier(&self) -> &str;
}

// Helper methods.

/// Convert a [`Vec`] of [`i16`]s into [`TimezoneIds`].
pub fn i16_vec_to_tomezoneids(value: Vec<i16>) -> TimezoneIds {
    if value.len() > TIMEZONE_LOOKUP_LENGTH {
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
