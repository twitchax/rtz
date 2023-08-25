//! Them for supporting the response types of the APIs, and their versions.

use axum::{response::{IntoResponse, Response}, http::HeaderValue};
use hyper::StatusCode;
use serde::Serialize;

use super::types::get_last_modified_time;

/// The response type for the [`get_timezone`] endpoint.
pub enum LookupResponse<T> {
    Ok(axum::Json<T>),
    NotModified,
    #[allow(dead_code)]
    NotFound,
}

impl<T> IntoResponse for LookupResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut response = match self {
            LookupResponse::Ok(json) => json.into_response(),
            LookupResponse::NotModified => (StatusCode::NOT_MODIFIED, "Not Modified").into_response(),
            LookupResponse::NotFound => {
                let body = r#"No timezone results: location likely resides on a boundary."#;

                (StatusCode::NOT_FOUND, body).into_response()
            }
        };

        response.headers_mut().insert("Last-Modified", HeaderValue::from_str(get_last_modified_time()).unwrap());
        response.headers_mut().insert("If-Modified-Since", HeaderValue::from_str(get_last_modified_time()).unwrap());

        response
    }
}
