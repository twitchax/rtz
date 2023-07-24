//! The `tz` module contains the timezone lookup functionality.
//!
//! This module will eventually support multiple different data sources, but for now
//! it only supports the Natural Earth Data (NED) source.

#[cfg(feature = "tz-ned")]
pub mod ned;
