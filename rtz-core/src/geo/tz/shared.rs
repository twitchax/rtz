//! The `shared` module.  Contains types and helpers pertinent to all TZ implementations.

// This module is mostly used for cache preprocessing, which is expensive during coverage, so
// it is not included in the coverage report.
#![cfg(not(tarpaulin_include))]

use crate::geo::shared::{HasGeometry, HasProperties};

// Types.

// Traits.

/// A trait for types that are a timezone and have a [`Geometry`].
///
/// Helps abstract away this property so the helper methods can be generalized.
pub trait IsTimezone: HasGeometry + HasProperties {
    /// Get the `identifier` of the [`IsTimezone`].
    fn identifier(&self) -> &str;
}