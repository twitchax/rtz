use std::{fmt::Display, io::Cursor, sync::OnceLock};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rocket::{
    http::{ContentType, Status},
    request::{FromRequest, Outcome},
    response::{self, status::Custom, Responder},
    serde::json::Json,
    Request, Response,
};
use rocket_okapi::{
    gen::OpenApiGenerator,
    okapi::openapi3::Responses,
    request::{OpenApiFromRequest, RequestHeaderInput},
    response::OpenApiResponderInner,
    JsonSchema, OpenApiError,
};
use serde::{Deserialize, Serialize};

use super::config::Config;
use crate::base::types::{Err, Timezone};

// Constants.

static LAST_MODIFIED_TIME: OnceLock<String> = OnceLock::new();

pub fn get_last_modified_time() -> &'static str {
    LAST_MODIFIED_TIME.get_or_init(|| {
        let exe_location = std::env::current_exe().unwrap();
        let exe_metadata = std::fs::metadata(exe_location).unwrap();
        let exe_modified = exe_metadata.modified().unwrap();

        DateTime::<Utc>::from(exe_modified).to_rfc2822().replace("+0000", "GMT")
    })
}

// Primary types.

/// The response type for the [`get_timezone`] endpoint.
pub enum TimezoneResponse {
    Ok(Json<TimezoneResponseRef>),
    NotModified,
    NotFound,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for TimezoneResponse {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let mut response = match self {
            TimezoneResponse::Ok(json) => json.respond_to(req)?,
            TimezoneResponse::NotModified => Response::build().status(Status::NotModified).finalize(),
            TimezoneResponse::NotFound => {
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
        response.set_raw_header("Expires", (Utc::now() + Duration::days(10)).to_rfc2822().replace("+0000", "GMT"));

        Ok(response)
    }
}

impl OpenApiResponderInner for TimezoneResponse {
    fn responses(generator: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiReponse};

        let mut responses = rocket_okapi::okapi::Map::new();

        let json_responses = Json::<TimezoneResponseRef>::responses(generator)?;

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
                This response is given when you request a page that does not exists.\
                "
                .to_string(),
                ..Default::default()
            }),
        );

        Ok(Responses { responses, ..Default::default() })
    }
}

/// The response type for the [`get_timezone`] endpoint when found.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimezoneResponseRef {
    /// The index of the [`Timezone`] in the global static cache.
    pub id: usize,
    /// The `objectid` of the [`Timezone`].
    pub objectid: u64,
    /// The `friendly_name` of the [`Timezone`] (e.g., `America/Los_Angeles`).
    pub friendly_name: Option<&'static str>,

    /// The `description` of the [`Timezone`] (e.g., the countries affected).
    pub description: &'static str,
    /// The `dst_description` of the [`Timezone`] (i.e., daylight savings time information).
    pub dst_description: Option<&'static str>,

    /// The `offset_str` of the [`Timezone`] (e.g., `UTC-8:00`).
    pub offset_str: &'static str,

    /// The `zone_num` of the [`Timezone`] (e.g., `-8`).
    pub zone_num: Option<i64>,
    /// The `zone_str` of the [`Timezone`] (e.g., `"-9.5"`).
    pub zone_str: &'static str,
    /// The `raw_offset` of the [`Timezone`] (e.g., `-28800`).
    pub raw_offset: i64,
}

impl From<&'static Timezone> for TimezoneResponseRef {
    fn from(value: &'static Timezone) -> TimezoneResponseRef {
        TimezoneResponseRef {
            id: value.id,
            objectid: value.objectid,
            friendly_name: value.friendly_name.as_deref(),
            description: value.description.as_ref(),
            dst_description: value.dst_description.as_deref(),
            offset_str: value.offset_str.as_ref(),
            zone_num: value.zone_num,
            zone_str: value.zone_str.as_ref(),
            raw_offset: value.raw_offset,
        }
    }
}

/// Holds the application-wide state for the Rocket web application.
///
/// See [Rocket State](https://rocket.rs/v0.4/guide/state/) for more information.
pub struct RocketState {
    pub config: Config,
}

// Helper types.

/// A request guard to get Last-Modified data.
#[derive(Debug)]
pub struct IfModifiedSince<'r>(&'r str);

#[async_trait]
impl<'r> FromRequest<'r> for IfModifiedSince<'r> {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let maybe_lm = request.headers().get_one("If-Modified-Since").map(IfModifiedSince);

        match maybe_lm {
            Some(lm) => Outcome::Success(lm),
            None => Outcome::Success(IfModifiedSince("NONE")),
        }
    }
}

impl<'r> OpenApiFromRequest<'r> for IfModifiedSince<'r> {
    fn from_request_input(_gen: &mut OpenApiGenerator, _name: String, _required: bool) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}

impl<'r> From<IfModifiedSince<'r>> for &'r str {
    fn from(val: IfModifiedSince<'r>) -> Self {
        val.0
    }
}

// Web types.

/// A simple web result with a custom error string.
pub type WebResult<T> = Result<T, WebError>;

/// A simple web result with no return value.
//pub type WebVoid = WebResult<()>;

/// A [`WebResult`] where the content is [`Json`].
//pub type JsonResult<T> = WebResult<Json<T>>;

// Web error types.

/// The error type returned during an HTTP error response.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct WebError {
    pub status: u16,
    pub message: String,
    pub backtrace: Option<String>,
}

impl std::error::Error for WebError {}

impl Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl<'r, 'o> Responder<'r, 'o> for WebError
where
    'o: 'r,
{
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        let _ = request.local_cache(|| self.clone());
        Custom(Status { code: self.status }, Json(self)).respond_to(request)
    }
}

impl OpenApiResponderInner for WebError {
    fn responses(_generator: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiReponse};

        let mut responses = rocket_okapi::okapi::Map::new();

        responses.insert(
            "400".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                #### [400 Bad Request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/400)\n\
                The request given is wrongly formatted or data asked could not be fulfilled. \
                "
                .to_string(),
                ..Default::default()
            }),
        );
        responses.insert(
            "401".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                #### [401 Unauthorized](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/401)\n\
                The request requires authentication. \
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
                This response is given when you request a page that does not exists.\
                "
                .to_string(),
                ..Default::default()
            }),
        );
        responses.insert(
            "422".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                #### [422 Unprocessable Entity](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/422)\n\
                This response is given when you request body is not correctly formatted. \
                "
                .to_string(),
                ..Default::default()
            }),
        );
        responses.insert(
            "500".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                #### [500 Internal Server Error](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/500)\n\
                This response is given when something went wrong on the server. \
                "
                .to_string(),
                ..Default::default()
            }),
        );

        Ok(Responses { responses, ..Default::default() })
    }
}

// Response builders.

/// Builds a [`Custom`] response (usually for errors).
#[allow(dead_code)]
pub fn custom(status: Status, message: impl Into<String>) -> WebError {
    let error = anyhow::Error::msg(message.into());

    WebError {
        status: status.code,
        message: error.to_string(),
        backtrace: Some(error.backtrace().to_string()),
    }
}

/// Builds an error response.
#[allow(dead_code)]
fn build_err<T>(status: Status, message: impl Into<String>) -> WebResult<T> {
    Err(custom(status, message))
}

/// Builds a bad request (400) response.
#[allow(dead_code)]
pub fn bad_req<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(Status::BadRequest, message)
}

/// Builds an internal error (500) response.
#[allow(dead_code)]
pub fn internal_err<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(Status::InternalServerError, message)
}

/// Builds an unauthorized (401) response.
#[allow(dead_code)]
pub fn unauthorized<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(Status::Unauthorized, message)
}

/// Builds an unauthorized (404) response.
#[allow(dead_code)]
pub fn notfound<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(Status::NotFound, message)
}

/// Builds a function that takes an error, and maps it into an error response.
fn map_err(status: Status, message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    move |e| WebError {
        status: status.code,
        message: format!("{}  {}", message.into(), e),
        backtrace: Some(e.backtrace().to_string()),
    }
}

/// Builds a function that takes an error, and maps it into a bad request (400) response.
pub fn map_bad_req(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(Status::BadRequest, message)
}

/// Builds a function that takes an error, and maps it into an internal error (500) response.
pub fn map_internal_err(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(Status::InternalServerError, message)
}

/// Builds a function that takes an error, and maps it into an unauthorized (401) response.
pub fn map_unauthorized(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(Status::Unauthorized, message)
}

/// Builds a function that takes an error, and maps it into an not found (404) response.
pub fn map_notfound(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(Status::NotFound, message)
}

/// Builds a function that takes an error, and maps it into an not found (429) response.
pub fn map_too_many_requests(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(Status::TooManyRequests, message)
}

/// Trait that makes it easy to map generic `Result`s into `WebResult`s.
pub trait WebResultMapper<T> {
    fn or_bad_req(self, message: impl Into<String>) -> WebResult<T>;
    fn or_internal_err(self, message: impl Into<String>) -> WebResult<T>;
    fn or_unauthorized(self, message: impl Into<String>) -> WebResult<T>;
    fn or_notfound(self, message: impl Into<String>) -> WebResult<T>;
    fn or_too_many_requests(self, message: impl Into<String>) -> WebResult<T>;
}

impl<T, E> WebResultMapper<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn or_bad_req(self, message: impl Into<String>) -> WebResult<T> {
        self.map_err(|e| Err::from(e)).map_err(map_bad_req(message))
    }

    fn or_internal_err(self, message: impl Into<String>) -> WebResult<T> {
        self.map_err(move |e| Err::from(e)).map_err(map_internal_err(message))
    }

    fn or_unauthorized(self, message: impl Into<String>) -> WebResult<T> {
        self.map_err(move |e| Err::from(e)).map_err(map_unauthorized(message))
    }

    fn or_notfound(self, message: impl Into<String>) -> WebResult<T> {
        self.map_err(move |e| Err::from(e)).map_err(map_notfound(message))
    }

    fn or_too_many_requests(self, message: impl Into<String>) -> WebResult<T> {
        self.map_err(move |e| Err::from(e)).map_err(map_too_many_requests(message))
    }
}

impl<T> WebResultMapper<T> for Option<T> {
    fn or_bad_req(self, message: impl Into<String>) -> WebResult<T> {
        self.ok_or_else(|| anyhow::Error::msg("Option was `None`.")).map_err(map_bad_req(message))
    }

    fn or_internal_err(self, message: impl Into<String>) -> WebResult<T> {
        self.ok_or_else(|| anyhow::Error::msg("Option was `None`.")).map_err(map_internal_err(message))
    }

    fn or_unauthorized(self, message: impl Into<String>) -> WebResult<T> {
        self.ok_or_else(|| anyhow::Error::msg("Option was `None`.")).map_err(map_unauthorized(message))
    }

    fn or_notfound(self, message: impl Into<String>) -> WebResult<T> {
        self.ok_or_else(|| anyhow::Error::msg("Option was `None`.")).map_err(map_notfound(message))
    }

    fn or_too_many_requests(self, message: impl Into<String>) -> WebResult<T> {
        self.ok_or_else(|| anyhow::Error::msg("Option was `None`.")).map_err(map_too_many_requests(message))
    }
}
