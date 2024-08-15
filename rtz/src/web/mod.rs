//! The web module.

// Modules.

pub(crate) mod config;
pub(crate) mod response_types;
pub(crate) mod server;
pub(crate) mod types;
pub(crate) mod utilities;

// Imports.

use log::LevelFilter;
use simple_logger::SimpleLogger;

use rtz_core::base::types::Void;

use crate::web::config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Starts the web server.
pub fn server_start(config_path: String, bind_address: Option<String>, port: Option<u16>, should_log: Option<bool>) -> Void {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        // Spew version.

        println!("Version: v{}", VERSION);

        // Ingest config.

        let config = Config::new(&config_path, bind_address, port, should_log)?;

        // Set up logging.

        let log_level = if config.should_log { LevelFilter::Info } else { LevelFilter::Off };
        SimpleLogger::new().with_level(log_level).init().unwrap();

        // Start server.

        server::start(&config).await?;

        Ok::<_, rtz_core::base::types::Err>(())
    })?;

    Ok(())
}

// Tests.

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, Router};
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use pretty_assertions::assert_eq;
    use tower::{Service, ServiceExt};

    fn get_client() -> Router {
        let config = Config::new("", None, None, Some(false)).unwrap();

        server::create_axum_app(&config)
    }

    #[tokio::test]
    async fn can_get_ned_timezone_v1() {
        let client = get_client();

        let request = Request::get("/api/v1/ned/tz/-121.0/46.0").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap_or_default().to_bytes();
        let expected = r#"[{"id":20,"identifier":"America/Los_Angeles","description":"Canada (most of British Columbia), Mexico (Baja California), United States (California, most of Nevada, most of Oregon, Washington (state))","dstDescription":"Canada (most of British Columbia), Mexico (Baja California), United States (California, most of Nevada, most of Oregon, Washington (state))","offset":"UTC-08:00","zone":-8.0,"rawOffset":-28800}]"#;

        assert_eq!(body, expected);
    }

    #[tokio::test]
    async fn can_get_not_found_ned_timezone_v1() {
        let client = get_client();

        let request = Request::get("/api/v1/ned/tz/179.9968/-67.0959").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap_or_default().to_bytes();
        let expected = r#"[]"#;

        assert_eq!(body, expected);
    }

    #[tokio::test]
    async fn can_get_not_modified_ned_timezone_v1() {
        let mut client = get_client();

        let request = Request::get("/api/v1/ned/tz/-121.0/46.0").body(Body::empty()).unwrap();
        // This is required because there are multiple impls of `ready` for `Router`. 🙄
        let response = <axum::Router as tower::ServiceExt<Request<Body>>>::ready(&mut client).await.unwrap().call(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let if_modified_since = response.headers().get("If-Modified-Since").unwrap().as_bytes();

        let request = Request::get("/api/v1/ned/tz/-121.0/46.0").header("If-Modified-Since", if_modified_since).body(Body::empty()).unwrap();
        let response = <axum::Router as tower::ServiceExt<Request<Body>>>::ready(&mut client).await.unwrap().call(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn can_get_osm_timezone_v1() {
        let client = get_client();

        let request = Request::get("/api/v1/osm/tz/-112/33").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap_or_default().to_bytes();
        let expected = r#"[{"id":162,"identifier":"America/Phoenix","shortIdentifier":"MST","offset":"UTC-07:00","rawOffset":-25200,"rawBaseOffset":-25200,"rawDstOffset":0,"zone":-7.0"#;

        assert!(body.starts_with(expected.as_bytes()));
    }

    #[tokio::test]
    async fn can_get_osm_admin_v1() {
        let client = get_client();

        let request = Request::get("/api/v1/osm/admin/30/30").body(Body::empty()).unwrap();
        let response = client.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap_or_default().to_bytes();
        let expected = r#"[{"id":217,"name":"مصر","level":2},{"id":3007,"name":"مطروح","level":4}]"#;

        assert_eq!(body, expected);
    }
}
