use std::{
    fmt::Display,
    sync::{Arc, OnceLock},
};

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue},
    response::{IntoResponse, Response},
};
use axum_insights::AppInsightsError;
use chrono::{DateTime, Utc};
use hyper::{StatusCode, header};
use rtz_core::base::types::Err;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::config::Config;

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

/// Holds the application-wide state for the Rocket web application.
#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Arc<Config>,
}

// Helper types.

/// A request guard to get Last-Modified data.
#[derive(Debug)]
pub struct IfModifiedSince(HeaderValue);

#[async_trait]
impl<S> FromRequestParts<S> for IfModifiedSince
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .headers
            .remove("If-Modified-Since")
            .map(IfModifiedSince)
            .unwrap_or_else(|| IfModifiedSince(HeaderValue::from_str("UNKNOWN").unwrap())))
    }
}

impl IfModifiedSince {
    pub fn as_str(&self) -> &str {
        self.0.to_str().unwrap()
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
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct WebError {
    pub status: u16,
    pub message: String,
    pub backtrace: Option<String>,
}

impl Default for WebError {
    fn default() -> Self {
        WebError {
            status: 0,
            message: "An unknown error occurred.".to_string(),
            backtrace: None,
        }
    }
}

impl AppInsightsError for WebError {
    fn message(&self) -> Option<String> {
        Some(self.message.clone())
    }

    fn backtrace(&self) -> Option<String> {
        self.backtrace.clone()
    }
}

impl std::error::Error for WebError {}

impl Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let code = StatusCode::from_u16(self.status).unwrap();
        let body = serde_json::to_string(&self).unwrap();

        (
            code,
            [(header::CONTENT_TYPE, "application/json")],
            body
        ).into_response()
    }
}

// Response builders.

/// Builds a [`Custom`] response (usually for errors).
#[allow(dead_code)]
pub fn custom(status: StatusCode, message: impl Into<String>) -> WebError {
    let error = anyhow::Error::msg(message.into());

    WebError {
        status: status.as_u16(),
        message: error.to_string(),
        backtrace: Some(error.backtrace().to_string()),
    }
}

/// Builds an error response.
#[allow(dead_code)]
fn build_err<T>(status: StatusCode, message: impl Into<String>) -> WebResult<T> {
    Err(custom(status, message))
}

/// Builds a bad request (400) response.
#[allow(dead_code)]
pub fn bad_req<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(StatusCode::BAD_REQUEST, message)
}

/// Builds an internal error (500) response.
#[allow(dead_code)]
pub fn internal_err<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(StatusCode::INTERNAL_SERVER_ERROR, message)
}

/// Builds an unauthorized (401) response.
#[allow(dead_code)]
pub fn unauthorized<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(StatusCode::UNAUTHORIZED, message)
}

/// Builds an unauthorized (404) response.
#[allow(dead_code)]
pub fn notfound<T>(message: impl Into<String>) -> WebResult<T> {
    build_err(StatusCode::NOT_FOUND, message)
}

/// Builds a function that takes an error, and maps it into an error response.
fn map_err(status: StatusCode, message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    move |e| WebError {
        status: status.as_u16(),
        message: format!("{}  {}", message.into(), e),
        backtrace: Some(e.backtrace().to_string()),
    }
}

/// Builds a function that takes an error, and maps it into a bad request (400) response.
pub fn map_bad_req(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(StatusCode::BAD_REQUEST, message)
}

/// Builds a function that takes an error, and maps it into an internal error (500) response.
pub fn map_internal_err(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(StatusCode::INTERNAL_SERVER_ERROR, message)
}

/// Builds a function that takes an error, and maps it into an unauthorized (401) response.
pub fn map_unauthorized(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(StatusCode::UNAUTHORIZED, message)
}

/// Builds a function that takes an error, and maps it into an not found (404) response.
pub fn map_notfound(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(StatusCode::NOT_FOUND, message)
}

/// Builds a function that takes an error, and maps it into an not found (429) response.
pub fn map_too_many_requests(message: impl Into<String>) -> impl FnOnce(Err) -> WebError {
    map_err(StatusCode::TOO_MANY_REQUESTS, message)
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
