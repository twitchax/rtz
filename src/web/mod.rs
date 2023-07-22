//! The web module.

// Modules.

pub(crate) mod config;
pub(crate) mod server;
pub(crate) mod types;

// Imports.

use log::{info, LevelFilter};
use simple_logger::SimpleLogger;

use crate::{web::config::Config, Void};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Starts the web server.
#[cfg(feature = "web")]
pub fn server_start(config_path: String, bind_address: Option<String>, port: Option<u16>) -> Void {
    use crate::base::types::Err;

    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        // Set up logging.
        SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

        // Spew version.

        info!("Version: v{}", VERSION);

        // Ingest config.

        let config = Config::new(&config_path, bind_address, port)?;

        // Start rocket server.

        server::start(&config, true).await?;

        Ok::<_, Err>(())
    })?;

    Ok(())
}

// Tests.

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rocket::{local::asynchronous::Client, http::{Status, Header}};

    async fn get_client() -> Client {
        let config = Config::new("", None, None).unwrap();
    
        Client::tracked(super::server::create_rocket(&config, false).expect("Tests require that Rocket be successfully created."))
            .await
            .expect("Tests require that the Rocket client be instantiated.")
    }

    #[tokio::test]
    async fn can_get_timezone() {
        let client = get_client().await;

        let response = client.get("/api/tz/-121.0/46.0").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.unwrap();
        let expected = r#"{"id":20,"objectid":6,"friendlyName":"America/Los_Angeles","description":"Canada (most of British Columbia), Mexico (Baja California), United States (California, most of Nevada, most of Oregon, Washington (state))","dstDescription":"Canada (most of British Columbia), Mexico (Baja California), United States (California, most of Nevada, most of Oregon, Washington (state))","offsetStr":"UTC-08:00","zoneNum":-8,"zoneStr":"-8","rawOffset":-28800}"#;

        assert_eq!(body, expected);
    }

    #[tokio::test]
    async fn can_get_not_found_timezone() {
        let client = get_client().await;

        let response = client.get("/api/tz/179.9968/-67.0959").dispatch().await;

        assert_eq!(response.status(), Status::NotFound);

        let body = response.into_string().await.unwrap();
        let expected = r#"{"status": 404,"message": "No timezone results: location likely resides on a boundary."}"#;

        assert_eq!(body, expected);
    }

    #[tokio::test]
    async fn can_get_not_modified_timezone() {
        let client = get_client().await;

        let response = client.get("/api/tz/-121.0/46.0").dispatch().await;

        assert_eq!(response.status(), Status::Ok);
        
        let if_modified_since = response.headers().get_one("If-Modified-Since").unwrap().to_string();

        let response = client.get("/api/tz/-121.0/46.0").header(Header::new("If-Modified-Since", if_modified_since)).dispatch().await;

        assert_eq!(response.status(), Status::NotModified);
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use rocket::{local::blocking::Client, http::Status};
    use test::{Bencher};

    use super::config::Config;

    fn get_client() -> Client {
        let config = Config::new("", None, None).unwrap();
    
        Client::tracked(super::server::create_rocket(&config, false).expect("Tests require that Rocket be successfully created."))
            .expect("Tests require that the Rocket client be instantiated.")
    }

    #[bench]
    fn bench_server_sweep(b: &mut Bencher) {
        let client = get_client();
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    let response = client.get(format!("/api/tz/{}/{}", x, y)).dispatch();
                    assert_eq!(response.status(), Status::Ok);
                }
            }
        });
    }
    
    #[bench]
    fn bench_server_worst_case_single(b: &mut Bencher) {
        let client = get_client();
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            let response = client.get(format!("/api/tz/{}/{}", x, y)).dispatch();
                    assert_eq!(response.status(), Status::Ok);
        });
    }

    
}
