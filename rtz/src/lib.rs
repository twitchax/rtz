//! A tool to easily work with timezone lookups.
//!
//! # Examples
#![cfg_attr(
    feature = "tz-ned",
    doc = r##"
```
use rtzlib::NedTimezone;
use rtzlib::CanPerformGeoLookup;

// Query a time zone for a given `(lng,lat)`.
assert_eq!(
    NedTimezone::lookup(-121., 46.)[0]
        .identifier
        .as_ref()
        .unwrap(),
    "America/Los_Angeles"
);
```
"##
)]
// Directives.
#![warn(rustdoc::broken_intra_doc_links, rust_2018_idioms, clippy::all, missing_docs)]
#![allow(incomplete_features)]
#![feature(async_closure)]
#![feature(test)]
#![feature(string_remove_matches)]
#![feature(fs_try_exists)]
#![allow(stable_features)]
#![feature(once_cell)]

// Modules.

pub mod shared;
pub mod geo;
pub use crate::geo::shared::CanPerformGeoLookup;

#[cfg(feature = "tz-ned")]
pub use rtz_core::geo::tz::ned::NedTimezone;

#[cfg(feature = "tz-osm")]
pub use rtz_core::geo::tz::osm::OsmTimezone;

#[cfg(feature = "admin-osm")]
pub use rtz_core::geo::admin::osm::OsmAdmin;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "web")]
pub mod web;
#[cfg(feature = "web")]
pub use crate::web::server_start;
