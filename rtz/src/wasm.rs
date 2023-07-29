//! The WASM module.
//!
//! This module contains the WASM wrappers / bindings for the rest of the library.

use crate::CanPerformGeoLookup;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc<'_> = wee_alloc::WeeAlloc::INIT;

// [`Timezone`] ABI.

/// Get the time zone for the given `(lng,lat)`.
#[cfg(feature = "tz-ned")]
#[wasm_bindgen(js_name = getTimezoneNed)]
pub fn get_timezone_ned(lng: f32, lat: f32) -> JsValue {
    let tzs = crate::NedTimezone::lookup(lng, lat);
    JsValue::from_str(&serde_json::to_string(&tzs).unwrap())
}

/// Get the time zone for the given `(lng,lat)`.
#[cfg(feature = "tz-osm")]
#[wasm_bindgen(js_name = getTimezoneOsm)]
pub fn get_timezone_osm(lng: f32, lat: f32) -> JsValue {
    let tzs = crate::OsmTimezone::lookup(lng, lat);
    JsValue::from_str(&serde_json::to_string(&tzs).unwrap())
}
