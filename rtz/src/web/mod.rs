//! The web module.

// Modules.

pub(crate) mod config;
pub(crate) mod response_types;
pub(crate) mod server;
pub(crate) mod types;

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

        // Start rocket server.

        server::start(&config).await?;

        Ok::<_, rtz_core::base::types::Err>(())
    })?;

    Ok(())
}

// Tests.

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rocket::{
        http::{Header, Status},
        local::asynchronous::Client,
    };

    async fn get_client() -> Client {
        let config = Config::new("", None, None, Some(false)).unwrap();

        Client::tracked(super::server::create_rocket(&config).expect("Tests require that Rocket be successfully created."))
            .await
            .expect("Tests require that the Rocket client be instantiated.")
    }

    #[tokio::test]
    async fn can_get_ned_timezone_v1() {
        let client = get_client().await;

        let response = client.get("/api/v1/ned/tz/-121.0/46.0").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.unwrap();
        let expected = r#"{"id":20,"identifier":"America/Los_Angeles","description":"Canada (most of British Columbia), Mexico (Baja California), United States (California, most of Nevada, most of Oregon, Washington (state))","dstDescription":"Canada (most of British Columbia), Mexico (Baja California), United States (California, most of Nevada, most of Oregon, Washington (state))","offset":"UTC-08:00","zone":-8.0,"rawOffset":-28800}"#;

        assert_eq!(body, expected);
    }

    #[tokio::test]
    async fn can_get_not_found_ned_timezone_v1() {
        let client = get_client().await;

        let response = client.get("/api/v1/ned/tz/179.9968/-67.0959").dispatch().await;

        assert_eq!(response.status(), Status::NotFound);

        let body = response.into_string().await.unwrap();
        let expected = r#"{"status": 404,"message": "No timezone results: location likely resides on a boundary."}"#;

        assert_eq!(body, expected);
    }

    #[tokio::test]
    async fn can_get_not_modified_ned_timezone_v1() {
        let client = get_client().await;

        let response = client.get("/api/v1/ned/tz/-121.0/46.0").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let if_modified_since = response.headers().get_one("If-Modified-Since").unwrap().to_string();

        let response = client.get("/api/v1/ned/tz/-121.0/46.0").header(Header::new("If-Modified-Since", if_modified_since)).dispatch().await;

        assert_eq!(response.status(), Status::NotModified);
    }

    #[tokio::test]
    async fn can_get_osm_timezone_v1() {
        let client = get_client().await;

        let response = client.get("/api/v1/osm/tz/-112/33").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.unwrap();
        let expected = r#"[{"id":162,"identifier":"America/Phoenix","shortIdentifier":"MST","offset":"UTC-07:00","rawOffset":-25200,"rawBaseOffset":-25200,"rawDstOffset":0,"zone":-7.0"#;

        assert!(body.starts_with(expected));
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use rocket::{http::Status, local::blocking::Client};
    use test::Bencher;

    use super::config::Config;

    fn get_client() -> Client {
        let config = Config::new("", None, None, Some(false)).unwrap();

        Client::tracked(super::server::create_rocket(&config).expect("Tests require that Rocket be successfully created.")).expect("Tests require that the Rocket client be instantiated.")
    }

    #[bench]
    fn bench_server_sweep_ned_v1(b: &mut Bencher) {
        let client = get_client();
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    let response = client.get(format!("/api/ned/tz/{}/{}", x, y)).dispatch();
                    assert_eq!(response.status(), Status::Ok);
                }
            }
        });
    }

    #[bench]
    fn bench_server_worst_case_single_ned_v1(b: &mut Bencher) {
        let client = get_client();
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            let response = client.get(format!("/api/ned/tz/{}/{}", x, y)).dispatch();
            assert_eq!(response.status(), Status::Ok);
        });
    }

    #[bench]
    fn bench_server_sweep_osm_v1(b: &mut Bencher) {
        let client = get_client();
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    let response = client.get(format!("/api/osm/tz/{}/{}", x, y)).dispatch();
                    assert_eq!(response.status(), Status::Ok);
                }
            }
        });
    }

    #[bench]
    fn bench_server_worst_case_single_osm_v1(b: &mut Bencher) {
        let client = get_client();
        let x = -86.5;
        let y = 38.5;

        b.iter(|| {
            let response = client.get(format!("/api/osm/tz/{}/{}", x, y)).dispatch();
            assert_eq!(response.status(), Status::Ok);
        });
    }
}
