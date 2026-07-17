//! The `shared` module.  Contains types and helpers pertinent to all TZ implementations.

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
