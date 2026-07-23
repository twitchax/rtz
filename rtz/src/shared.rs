//! Shared functionality for the `rtz` crate.

#[cfg(any(feature = "admin-osm", feature = "tz-ned", feature = "tz-osm"))]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use utoipa::ToSchema;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg(feature = "admin-osm")]
use rtz_core::geo::admin::osm::OsmAdmin;
#[cfg(feature = "tz-ned")]
use rtz_core::geo::tz::ned::NedTimezone;
#[cfg(feature = "tz-osm")]
use rtz_core::geo::tz::osm::OsmTimezone;

/// The response type for the NED timezone endpoint when found.
///
/// Currently ingested version of this data set is [here](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_time_zones.geojson).
#[cfg(feature = "tz-ned")]
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(ToSchema))]
#[cfg_attr(feature = "wasm", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct NedTimezoneResponse1 {
    /// The index of this timezone in the global static cache.
    ///
    /// This is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The IANA time zone identifier (e.g., `America/Los_Angeles`).
    pub identifier: Option<&'static str>,

    /// The countries and regions this zone covers.
    pub description: &'static str,
    /// The daylight savings time information for this zone.
    pub dst_description: Option<&'static str>,

    /// The UTC offset in display form (e.g., `UTC-8:00`).
    pub offset: &'static str,

    /// The UTC offset in hours (e.g., `-8`).
    pub zone: f32,
    /// The UTC offset in seconds (e.g., `-28800`).
    pub raw_offset: i32,
}

#[cfg(feature = "tz-ned")]
impl From<&'static NedTimezone> for NedTimezoneResponse1 {
    fn from(value: &'static NedTimezone) -> NedTimezoneResponse1 {
        NedTimezoneResponse1 {
            id: value.id,
            identifier: value.identifier.as_deref(),
            description: value.description.as_ref(),
            dst_description: value.dst_description.as_deref(),
            offset: value.offset.as_ref(),
            zone: value.zone,
            raw_offset: value.raw_offset,
        }
    }
}

/// The response type for the OSM timezone endpoint when found.
///
/// Currently ingested version of this data set is [here](https://github.com/evansiroky/timezone-boundary-builder/releases/download/2023b/timezones-with-oceans.geojson.zip).
#[cfg(feature = "tz-osm")]
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(ToSchema))]
#[cfg_attr(feature = "wasm", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct OsmTimezoneResponse1 {
    /// The index of this timezone in the global static cache.
    ///
    /// This is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The IANA time zone identifier (e.g., `America/Los_Angeles`).
    pub identifier: &'static str,
    /// The abbreviated name of the offset currently in effect (e.g., `PDT`).
    pub short_identifier: String,

    /// The current UTC offset in display form (e.g., `UTC-8:00`).
    pub offset: String,

    /// The current UTC offset in seconds, including any daylight savings adjustment (e.g., `-28800`).
    pub raw_offset: i32,
    /// The standard UTC offset in seconds, excluding daylight savings (e.g., `-28800`).
    pub raw_base_offset: i32,
    /// The daylight savings adjustment in seconds, or `0` when it is not in effect.
    pub raw_dst_offset: i32,

    /// The current UTC offset in hours (e.g., `-8`).
    pub zone: f32,

    /// The current time in this timezone, as an RFC 3339 timestamp.
    pub current_time: String,
}

#[cfg(feature = "tz-osm")]
impl From<&'static OsmTimezone> for OsmTimezoneResponse1 {
    fn from(value: &'static OsmTimezone) -> OsmTimezoneResponse1 {
        use chrono::{Offset, Utc};
        use chrono_tz::{OffsetComponents, Tz};

        let tz: Tz = value.identifier.parse().unwrap();
        let time = Utc::now().with_timezone(&tz);
        let tz_offset = time.offset();
        let fixed_offset = tz_offset.fix();

        let short_identifier = tz_offset.to_string();

        let offset = format!("UTC{}", fixed_offset);
        let raw_offset = fixed_offset.local_minus_utc();
        let raw_base_offset = tz_offset.base_utc_offset().num_seconds() as i32;
        let raw_dst_offset = tz_offset.dst_offset().num_seconds() as i32;

        let zone = raw_offset as f32 / 3600.0;

        let current_time = time.to_rfc3339();

        OsmTimezoneResponse1 {
            id: value.id,
            identifier: value.identifier.as_ref(),
            short_identifier,
            offset,
            raw_offset,
            raw_base_offset,
            raw_dst_offset,
            zone,
            current_time,
        }
    }
}

/// The response type for the OSM admin endpoint when found.
///
/// Results are returned broadest-first: ascending by `level`, so a point inside nested areas
/// yields the containment hierarchy in order (country, then state, then county, then city).
///
/// Currently ingested version of this data set is [here](https://planet.openstreetmap.org/).
#[cfg(feature = "admin-osm")]
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(ToSchema))]
#[cfg_attr(feature = "wasm", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct OsmAdminResponse1 {
    /// The index of this admin area in the global static cache.
    ///
    /// This is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,

    /// The OSM relation id of the admin area (e.g., `1473947`), or `null` if the source boundary
    /// was not relation-backed.  Unlike `id`, this is stable across builds.
    pub relation_id: Option<u64>,

    /// The name of the admin area (e.g., `France`).
    pub name: &'static str,

    /// The OSM admin level of the area (e.g., `2` for a country).
    pub level: usize,
}

#[cfg(feature = "admin-osm")]
impl From<&'static OsmAdmin> for OsmAdminResponse1 {
    fn from(value: &'static OsmAdmin) -> OsmAdminResponse1 {
        OsmAdminResponse1 {
            id: value.id,
            relation_id: (value.relation_id != 0).then_some(value.relation_id),
            name: value.name.as_ref(),
            level: value.level,
        }
    }
}
