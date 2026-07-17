use std::{
    fmt::Display,
    sync::{Arc, OnceLock},
};

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue},
    response::{IntoResponse, Response},
};
use axum_insights::AppInsightsError;
use chrono::{DateTime, Utc};
use hyper::{header, StatusCode};
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
    #[allow(dead_code)]
    pub config: Arc<Config>,
}

// Helper types.

/// A request guard to get Last-Modified data.
#[derive(Debug)]
pub struct IfModifiedSince(HeaderValue);

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
pub type WebVoid = WebResult<()>;

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

        (code, [(header::CONTENT_TYPE, "application/json")], body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn web_error_display_is_the_message() {
        let e = WebError { status: 400, message: "boom".to_string(), backtrace: None };
        assert_eq!(e.to_string(), "boom");
    }

    #[test]
    fn web_error_into_response_uses_its_status() {
        let e = WebError { status: 404, message: "nope".to_string(), backtrace: None };
        let response = e.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn last_modified_time_is_nonempty() {
        assert!(!get_last_modified_time().is_empty());
    }
}
