//! The `tz` module contains the timezone lookup functionality.

pub mod shared;

#[cfg(feature = "tz-ned")]
pub mod ned;

#[cfg(feature = "tz-osm")]
pub mod osm;
