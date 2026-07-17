//! The `shared` module.  Contains types and helpers pertinent to all admin implementations.

use crate::geo::shared::{HasGeometry, HasProperties};

// Types.

// Traits.

/// A trait for types that are an admin and have a [`Geometry`].
///
/// Helps abstract away this property so the helper methods can be generalized.
pub trait IsAdmin: HasGeometry + HasProperties {
    /// Get the `name` of the [`IsAdmin`].
    fn name(&self) -> &str;
}
