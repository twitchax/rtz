//! A tool to easily work with timezone lookups.
//!
//! # Examples
//! 
#![cfg_attr(feature = "tz-ned", doc = r##"
```
use rtzlib::get_timezone_ned;

// Query a time zone for a given `(lng,lat)`.
assert_eq!(
    get_timezone_ned(-121., 46.)
        .unwrap()
        .friendly_name
        .as_ref()
        .unwrap(),
    "America/Los_Angeles"
);
```
"##)]

// Directives.

#![warn(rustdoc::broken_intra_doc_links, rust_2018_idioms, clippy::all, missing_docs)]
#![allow(incomplete_features)]
#![feature(async_closure)]
#![feature(test)]
#![feature(string_remove_matches)]
#![feature(fs_try_exists)]

// Modules.

pub mod geo;

#[cfg(feature = "tz-ned")]
pub use crate::geo::tz::ned::get_timezone as get_timezone_ned;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "web")]
pub mod web;
#[cfg(feature = "web")]
pub use crate::web::server_start;
