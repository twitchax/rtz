//! Them for supporting the response types of the APIs, and their versions.

use std::io::Cursor;

use chrono::{Offset, Utc};
use chrono_tz::{OffsetComponents, Tz};
use rocket::{
    http::{ContentType, Status},
    response::{self, Responder},
    serde::json::Json,
    Request, Response,
};
use rocket_okapi::{gen::OpenApiGenerator, okapi::openapi3::Responses, response::OpenApiResponderInner, OpenApiError};
use rtz_core::geo::{tz::{ned::NedTimezone, osm::OsmTimezone}, admin::osm::OsmAdmin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::get_last_modified_time;

/// The response type for the [`get_timezone`] endpoint.
pub enum LookupResponse<T> {
    Ok(Json<T>),
    NotModified,
    #[allow(dead_code)]
    NotFound,
}

impl<'r, 'o: 'r, T> Responder<'r, 'o> for LookupResponse<T>
where
    T: Serialize,
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let mut response = match self {
            LookupResponse::Ok(json) => json.respond_to(req)?,
            LookupResponse::NotModified => Response::build().status(Status::NotModified).finalize(),
            LookupResponse::NotFound => {
                let body = r#"{"status": 404,"message": "No timezone results: location likely resides on a boundary."}"#;
                Response::build()
                    .status(Status::NotFound)
                    .header(ContentType::JSON)
                    .sized_body(body.len(), Cursor::new(body))
                    .finalize()
            }
        };

        response.set_raw_header("Last-Modified", get_last_modified_time());
        response.set_raw_header("If-Modified-Since", get_last_modified_time());
        //response.set_raw_header("Expires", (Utc::now() + Duration::days(10)).to_rfc2822().replace("+0000", "GMT"));

        Ok(response)
    }
}

impl<T> OpenApiResponderInner for LookupResponse<T>
where
    T: Serialize + JsonSchema + Send,
{
    fn responses(generator: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiReponse};

        let mut responses = rocket_okapi::okapi::Map::new();

        let json_responses = Json::<T>::responses(generator)?;

        responses.extend(json_responses.responses);

        responses.insert(
            "304".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                #### [304 Not Modified](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/304)\n\
                The request result has not been modified. \
                "
                .to_string(),
                ..Default::default()
            }),
        );

        responses.insert(
            "404".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                #### [404 Not Found](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/404)\n\
                This response is given when you request a lookup that does not produce any results (possibly on the edge of a boundary). \
                "
                .to_string(),
                ..Default::default()
            }),
        );

        Ok(Responses { responses, ..Default::default() })
    }
}

/// The response type for the NED timezone endpoint when found.
///
/// Currently ingested version of this data set is [here](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_time_zones.geojson).
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NedTimezoneResponse1 {
    /// The index of the [`NedTimezoneResponse1`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The `identifier` of the [`NedTimezoneResponse1`] (e.g., `America/Los_Angeles`).
    ///
    /// Essentially, it is the IANA TZ identifier.
    pub identifier: Option<&'static str>,

    /// The `description` of the [`NedTimezoneResponse1`] (e.g., the countries affected).
    pub description: &'static str,
    /// The `dst_description` of the [`NedTimezoneResponse1`] (i.e., daylight savings time information).
    pub dst_description: Option<&'static str>,

    /// The `offset_str` of the [`NedTimezoneResponse1`] (e.g., `UTC-8:00`).
    pub offset: &'static str,

    /// The `zone_num` of the [`NedTimezoneResponse1`] (e.g., `-8`).
    pub zone: f32,
    /// The `raw_offset` of the [`NedTimezoneResponse1`] (e.g., `-28800`).
    pub raw_offset: i32,
}

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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OsmTimezoneResponse1 {
    /// The index of the [`OsmTimezoneResponse1`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The `identifier` of the [`OsmTimezoneResponse1`] (e.g., `America/Los_Angeles`).
    ///
    /// Essentially, it is the IANA TZ identifier.
    pub identifier: &'static str,
    /// The `short_identifier` of the [`OsmTimezoneResponse1`] (e.g., `PDT`).
    pub short_identifier: String,

    /// The `offset` of the [`OsmTimezoneResponse1`] (e.g., `UTC-8:00`).
    pub offset: String,

    /// The `raw_offset` of the [`OsmTimezoneResponse1`] (e.g., `-28800`).
    pub raw_offset: i32,
    /// The `raw_base_offset` of the [`OsmTimezoneResponse1`] (e.g., `-28800`).
    pub raw_base_offset: i32,
    /// The `raw_dst_offset` of the [`OsmTimezoneResponse1`] (e.g., `-28800`).
    pub raw_dst_offset: i32,

    /// The `zone_num` of the [`OsmTimezoneResponse1`] (e.g., `-8`).
    pub zone: f32,

    /// The current time in the timezone.
    pub current_time: String,
}

impl From<&'static OsmTimezone> for OsmTimezoneResponse1 {
    fn from(value: &'static OsmTimezone) -> OsmTimezoneResponse1 {
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
            identifier: value.identifier.as_str(),
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

/// The response type for the [`get_timezone`] endpoint when found.
///
/// Currently ingested version of this data set is [here](https://planet.openstreetmap.org/).
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OsmAdminResponse1 {
    /// The index of the [`OsmAdminResponse1`] in the global static cache.
    ///
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,

    /// The `name` of the [`OsmAdminResponse1`] (e.g., `France`).
    pub name: &'static str,
    
    /// The `admin_level` of the [`OsmAdminResponse1`] (e.g., `2`).
    pub level: u8,
}

impl From<&'static OsmAdmin> for OsmAdminResponse1 {
    fn from(value: &'static OsmAdmin) -> OsmAdminResponse1 {
        OsmAdminResponse1 {
            id: value.id,
            name: value.name.as_str(),
            level: value.level,
        }
    }
}
