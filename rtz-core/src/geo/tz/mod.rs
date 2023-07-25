//! The `tz` module that contains all of the timezone lookup abstractions.

pub mod shared;

#[cfg(feature = "tz-ned")]
pub mod ned;

#[cfg(feature = "tz-osm")]
pub mod osm;