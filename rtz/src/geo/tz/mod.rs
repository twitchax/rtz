//! The `tz` module contains the timezone lookup functionality.
//!
//! This module will eventually support multiple different data sources, but for now
//! it only supports the Natural Earth Data (NED) source.

pub mod shared;

#[cfg(feature = "tz-ned")]
pub mod ned;

#[cfg(feature = "tz-osm")]
pub mod osm;
