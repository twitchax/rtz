//! Them for supporting the response types of the APIs, and their versions.

use rocket::{
    http::{ContentType, Status},
    response::{self, Responder},
    serde::json::Json,
    Request, Response,
};
use rocket_okapi::{gen::OpenApiGenerator, okapi::openapi3::Responses, response::OpenApiResponderInner, OpenApiError};
use schemars::JsonSchema;
use serde::Serialize;
use std::io::Cursor;

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
