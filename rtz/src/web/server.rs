use std::net::IpAddr;

use rocket::{data::Limits, get, log::LogLevel, serde::json::Json, shield::Shield, Build, Rocket};
use rocket_okapi::{
    openapi, openapi_get_routes,
    swagger_ui::{make_swagger_ui, SwaggerUIConfig},
};
use rtz_core::base::types::{Float, Res, Void};

use super::{
    config::Config,
    response_types::{LookupResponse, NedTimezoneResponse1, OsmTimezoneResponse1},
    types::{get_last_modified_time, IfModifiedSince, RocketState, WebResult},
};

/// Starts the web server.
pub async fn start(config: &Config) -> Void {
    create_rocket(config)?.launch().await?;
    Ok(())
}

/// Creates the [`Rocket`] instance that defines how the web application behaves.
///
/// This method:
/// * Passes the state into Rocket, so that it can be supplied to request handlers.
/// * Mounts the static files, which are built externally, and copied into the final application container.
/// * Mounts the request handlers defined in this module.
/// * Attaches a custom fairing  
pub fn create_rocket(config: &Config) -> Res<Rocket<Build>> {
    let log_level = if config.should_log { LogLevel::Normal } else { LogLevel::Off };
    let rocket_config = rocket::config::Config {
        address: config.bind_address.parse::<IpAddr>()?,
        port: config.port,
        // TODO: This is not ideal (but we use it for images, because I'm lazy) ...
        limits: Limits::default(),
        log_level,
        ..rocket::config::Config::debug_default()
    };

    let state = RocketState { config: config.clone() };

    let rocket = rocket::custom(rocket_config)
        .manage(state)
        .mount("/api", openapi_get_routes![get_timezone_ned, get_timezone_ned_v1, get_timezone_osm, get_timezone_osm_v1])
        .mount(
            "/app-docs",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../api/openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .attach(Shield::new());

    Ok(rocket)
}

/// Returns the time zone information for the given `(lng,lat)` from the [Natural Earth Data](https://www.naturalearthdata.com/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[openapi(tag = "TZ")]
#[get("/ned/tz/<lng>/<lat>")]
async fn get_timezone_ned(lng: Float, lat: Float, if_modified_since: IfModifiedSince<'_>) -> WebResult<LookupResponse<NedTimezoneResponse1>> {
    get_timezone_ned_v1(lng, lat, if_modified_since).await
}

/// Returns the time zone information for the given `(lng,lat)` from the [Natural Earth Data](https://www.naturalearthdata.com/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[openapi(tag = "TZv1")]
#[get("/v1/ned/tz/<lng>/<lat>")]
async fn get_timezone_ned_v1(lng: Float, lat: Float, if_modified_since: IfModifiedSince<'_>) -> WebResult<LookupResponse<NedTimezoneResponse1>> {
    if Into::<&str>::into(if_modified_since) == get_last_modified_time() {
        log::warn!("Not modified.");
        return Ok(LookupResponse::NotModified);
    }

    let tz = match crate::get_timezone_ned(lng, lat) {
        Some(tz) => tz.into(),
        None => {
            log::warn!("Not found.");
            return Ok(LookupResponse::NotFound);
        }
    };

    Ok(LookupResponse::Ok(Json(tz)))
}

/// Returns the time zone information for the given `(lng,lat)` from the [OpenStreetMap](https://www.openstreetmap.org/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[openapi(tag = "TZ")]
#[get("/osm/tz/<lng>/<lat>")]
async fn get_timezone_osm(lng: Float, lat: Float) -> WebResult<LookupResponse<Vec<OsmTimezoneResponse1>>> {
    get_timezone_osm_v1(lng, lat).await
}

/// Returns the time zone information for the given `(lng,lat)` from the [OpenStreetMap](https://www.openstreetmap.org/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[openapi(tag = "TZv1")]
#[get("/v1/osm/tz/<lng>/<lat>")]
async fn get_timezone_osm_v1(lng: Float, lat: Float) -> WebResult<LookupResponse<Vec<OsmTimezoneResponse1>>> {
    let tzs = crate::get_timezones_osm(lng, lat).into_iter().map(|tz| tz.into()).collect::<Vec<_>>();

    Ok(LookupResponse::Ok(Json(tzs)))
}
