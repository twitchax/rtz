use std::net::IpAddr;

use rocket::{data::Limits, get, serde::json::Json, shield::Shield, Build, Rocket};
use rocket_okapi::{
    openapi, openapi_get_routes,
    swagger_ui::{make_swagger_ui, SwaggerUIConfig},
};

use crate::{base::types::Res, web::types::get_last_modified_time, Void};

use super::{
    config::Config,
    types::{IfModifiedSince, RocketState, TimezoneResponse, WebResult},
};

/// Starts the web server.
pub async fn start(config: &Config, should_log: bool) -> Void {
    create_rocket(config, should_log)?.launch().await?;
    Ok(())
}

/// Creates the [`Rocket`] instance that defines how the web application behaves.
///
/// This method:
/// * Passes the state into Rocket, so that it can be supplied to request handlers.
/// * Mounts the static files, which are built externally, and copied into the final application container.
/// * Mounts the request handlers defined in this module.
/// * Attaches a custom fairing  
pub fn create_rocket(config: &Config, should_log: bool) -> Res<Rocket<Build>> {
    let log_level = if should_log {
        rocket::config::LogLevel::Normal
    } else {
        rocket::config::LogLevel::Off
    };

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
        .mount("/api", openapi_get_routes![get_timezone])
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

#[openapi(tag = "API")]
#[get("/tz/<lng>/<lat>")]
async fn get_timezone(lng: f64, lat: f64, if_modified_since: IfModifiedSince<'_>) -> WebResult<TimezoneResponse> {
    if Into::<&str>::into(if_modified_since) == get_last_modified_time() {
        log::warn!("Not modified.");
        return Ok(TimezoneResponse::NotModified);
    }

    let tz = match crate::get_timezone(lng, lat) {
        Some(tz) => tz.into(),
        None => {
            log::warn!("Not found.");
            return Ok(TimezoneResponse::NotFound);
        }
    };

    Ok(TimezoneResponse::Ok(Json(tz)))
}
