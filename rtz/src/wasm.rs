//! The WASM module.
//!
//! This module contains the WASM wrappers / bindings for the rest of the library.

use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc<'_> = wee_alloc::WeeAlloc::INIT;

// [`Timezone`] ABI.

/// Get the time zone for the given `(lng,lat)`.
#[cfg(feature = "tz-ned")]
#[wasm_bindgen(js_name = getTimezoneNed)]
pub fn get_timezone_ned(lng: f64, lat: f64) -> JsValue {
    match crate::get_timezone_ned(lng, lat) {
        Some(tz) => JsValue::from_str(&serde_json::to_string(&tz).unwrap()),
        None => JsValue::NULL,
    }
}
