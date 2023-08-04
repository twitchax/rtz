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
    let tzs = crate::NedTimezone::lookup(lng, lat).into_iter().map(crate::shared::NedTimezoneResponse1::from).collect::<Vec<_>>();
    JsValue::from_str(&serde_json::to_string(&tzs).unwrap())
}

/// Get the time zone for the given `(lng,lat)`.
#[cfg(feature = "tz-osm")]
#[wasm_bindgen(js_name = getTimezoneOsm)]
pub fn get_timezone_osm(lng: f32, lat: f32) -> JsValue {
    let tzs = crate::OsmTimezone::lookup(lng, lat).into_iter().map(crate::shared::OsmTimezoneResponse1::from).collect::<Vec<_>>();
    JsValue::from_str(&serde_json::to_string(&tzs).unwrap())
}

// Admin ABI.

/// Get the admin for the given `(lng,lat)`.
#[cfg(feature = "admin-osm")]
#[wasm_bindgen(js_name = getAdminOsm)]
pub fn get_admin_osm(lng: f32, lat: f32) -> JsValue {
    let admins = crate::OsmAdmin::lookup(lng, lat).into_iter().map(crate::shared::OsmAdminResponse1::from).collect::<Vec<_>>();
    JsValue::from_str(&serde_json::to_string(&admins).unwrap())
}
