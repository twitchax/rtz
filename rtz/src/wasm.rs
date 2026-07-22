//! The WASM module.
//!
//! This module contains the WASM wrappers / bindings for the rest of the library.
//!
//! Each binding returns a real JS array of objects (via [`serde_wasm_bindgen`]), and the
//! `Tsify` derives on the response types in [`crate::shared`] emit the matching TypeScript
//! declarations, so `unchecked_return_type` below names a type the consumer actually gets.
//!
//! Note that `admin-osm` is deliberately not exposed here: the admin dataset roughly triples
//! the size of the published NPM package, which is the binding consumers are most sensitive to.

use crate::CanPerformGeoLookup;
use wasm_bindgen::{prelude::wasm_bindgen, JsError, JsValue};

/// Serialize a lookup result into a JS value, mapping any serialization failure into a thrown
/// JS exception rather than a panic (which, in WASM, traps and poisons the module instance).
fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}

// [`Timezone`] ABI.

/// Get the time zones for the given `(lng,lat)`.
#[cfg(feature = "tz-ned")]
#[wasm_bindgen(js_name = getTimezoneNed, unchecked_return_type = "NedTimezoneResponse1[]")]
pub fn get_timezone_ned(lng: f32, lat: f32) -> Result<JsValue, JsError> {
    let tzs = crate::NedTimezone::lookup(lng, lat).into_iter().map(crate::shared::NedTimezoneResponse1::from).collect::<Vec<_>>();
    to_js(&tzs)
}

/// Get the time zones for the given `(lng,lat)`.
#[cfg(feature = "tz-osm")]
#[wasm_bindgen(js_name = getTimezoneOsm, unchecked_return_type = "OsmTimezoneResponse1[]")]
pub fn get_timezone_osm(lng: f32, lat: f32) -> Result<JsValue, JsError> {
    let tzs = crate::OsmTimezone::lookup(lng, lat).into_iter().map(crate::shared::OsmTimezoneResponse1::from).collect::<Vec<_>>();
    to_js(&tzs)
}

// Admin ABI.

/// Get the admins for the given `(lng,lat)`.
#[cfg(feature = "admin-osm")]
#[wasm_bindgen(js_name = getAdminOsm, unchecked_return_type = "OsmAdminResponse1[]")]
pub fn get_admin_osm(lng: f32, lat: f32) -> Result<JsValue, JsError> {
    let admins = crate::OsmAdmin::lookup(lng, lat).into_iter().map(crate::shared::OsmAdminResponse1::from).collect::<Vec<_>>();
    to_js(&admins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Read a property off a JS value the same way a JS consumer would.
    fn prop(value: &JsValue, key: &str) -> JsValue {
        js_sys::Reflect::get(value, &JsValue::from_str(key)).expect("property read failed")
    }

    /// Regression: these bindings used to hand back a JSON *string* rather than a JS object, so
    /// the documented `tz[0].identifier` access silently evaluated to `undefined`. Nothing tested
    /// the ABI, so it went unnoticed across several releases. Assert the shape a consumer sees.
    #[cfg(feature = "tz-ned")]
    #[wasm_bindgen_test]
    fn ned_binding_returns_array_of_objects() {
        let value = get_timezone_ned(-121.0, 46.0).expect("lookup should serialize");

        assert!(js_sys::Array::is_array(&value), "expected a JS array, got a {:?}", value.js_typeof());

        let first = js_sys::Array::from(&value).get(0);
        assert_eq!(prop(&first, "identifier").as_string().as_deref(), Some("America/Los_Angeles"));

        // `rawOffset` must arrive as a JS number, not a stringified one.
        assert_eq!(prop(&first, "rawOffset").as_f64(), Some(-28800.0));
    }

    #[cfg(feature = "tz-osm")]
    #[wasm_bindgen_test]
    fn osm_binding_returns_array_of_objects() {
        let value = get_timezone_osm(30.0, 30.0).expect("lookup should serialize");

        assert!(js_sys::Array::is_array(&value), "expected a JS array, got a {:?}", value.js_typeof());

        let first = js_sys::Array::from(&value).get(0);
        assert_eq!(prop(&first, "identifier").as_string().as_deref(), Some("Africa/Cairo"));
    }
}
