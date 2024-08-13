use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use axum::{extract::Path, routing::get, Json, Router};
use axum_insights::AppInsights;
use http::{Method, StatusCode};
use rtz_core::{
    base::types::{Float, Void},
    geo::{
        admin::osm::OsmAdmin,
        tz::{ned::NedTimezone, osm::OsmTimezone},
    },
};
use tower_http::cors::{Any, CorsLayer};
use tracing::instrument;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    shared::{NedTimezoneResponse1, OsmAdminResponse1, OsmTimezoneResponse1},
    CanPerformGeoLookup,
};

use super::{
    config::Config,
    response_types::LookupResponse,
    types::{get_last_modified_time, AppState, IfModifiedSince, WebError, WebResult, WebVoid},
    utilities::shutdown_signal,
};

// Statics.

static FLY_REGION: OnceLock<String> = OnceLock::new();
static FLY_ALLOC_ID: OnceLock<String> = OnceLock::new();
static FLY_PUBLIC_IP: OnceLock<String> = OnceLock::new();

/// Starts the web server.
pub async fn start(config: &Config) -> Void {
    let app = create_axum_app(config);

    let bind_address = format!("{}:{}", config.bind_address, config.port);
    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.unwrap();

    Ok(())
}

pub fn create_axum_app(config: &Config) -> Router {
    let state = AppState { config: Arc::new(config.clone()) };

    let cors_layer = CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any);

    let name = std::env::var("FLY_REGION").unwrap_or_else(|_| "server".to_string());
    let _ = FLY_REGION.set(name.clone());
    let _ = FLY_ALLOC_ID.set(std::env::var("FLY_ALLOC_ID").unwrap_or_else(|_| "unknown".to_string()));
    let _ = FLY_PUBLIC_IP.set(std::env::var("FLY_PUBLIC_IP").unwrap_or_else(|_| "unknown".to_string()));

    let telemetry_layer = AppInsights::default()
        .with_connection_string(config.analytics_api_key.clone())
        .with_service_config("rtz", name)
        .with_live_metrics(true)
        .with_catch_panic(true)
        .with_field_mapper(|p| {
            let fly_alloc_id = FLY_ALLOC_ID.get().unwrap().to_owned();
            let fly_public_ip = FLY_PUBLIC_IP.get().unwrap().to_owned();
            let fly_region = FLY_REGION.get().unwrap().to_owned();
            let fly_accept_region = p.headers.get("Fly-Region").map(|v| v.to_str().unwrap_or("unknown").to_owned()).unwrap_or("unknown".to_owned());

            HashMap::from([
                ("fly.alloc_id".to_string(), fly_alloc_id),
                ("fly.public_ip".to_string(), fly_public_ip),
                ("fly.server_region".to_string(), fly_region),
                ("fly.accept_region".to_string(), fly_accept_region),
            ])
        })
        .with_panic_mapper(|e| {
            (
                500,
                WebError {
                    status: 500,
                    message: format!("A panic occurred: {:?}", e),
                    backtrace: None,
                },
            )
        })
        .with_noop(config.analytics_api_key.is_none())
        .with_success_filter(|status| status.is_success() || status.is_redirection() || status.is_informational() || status == StatusCode::NOT_FOUND)
        .with_error_type::<WebError>()
        .build_and_set_global_default()
        .unwrap()
        .layer();

    let api_router = Router::new()
        .route("/health", get(health))
        .route("/ned/tz/:lng/:lat", get(timezone_ned))
        .route("/v1/ned/tz/:lng/:lat", get(timezone_ned_v1))
        .route("/osm/tz/:lng/:lat", get(timezone_osm))
        .route("/v1/osm/tz/:lng/:lat", get(timezone_osm_v1))
        .route("/osm/admin/:lng/:lat", get(admin_osm))
        .route("/v1/osm/admin/:lng/:lat", get(admin_osm_v1));

    Router::new()
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        .nest("/api", api_router)
        .layer(cors_layer)
        .layer(telemetry_layer)
        .with_state(state)
}

#[derive(OpenApi)]
#[openapi(
    paths(health, timezone_ned, timezone_ned_v1, timezone_osm, timezone_osm_v1, admin_osm, admin_osm_v1),
    components(schemas(NedTimezoneResponse1, OsmTimezoneResponse1, OsmAdminResponse1))
)]
struct ApiDoc;

/// Performs a "semi-deep" health check.
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/health", 
    tag = "Health", 
    responses(
        (status = 200, description = "List all found timezones successfully.", body = ()),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No timezone results: location likely resides on a boundary."),
        (status = 500, description = "An unwarranted failure."),
    )
)]
#[instrument]
async fn health(if_modified_since: IfModifiedSince) -> WebVoid {
    timezone_ned_v1(Path((30.0, 30.0)), if_modified_since).await?;

    Ok(())
}

/// Gets time zone information from the NED dataset.
///
/// Returns the time zone information for the given `(lng,lat)` from the [Natural Earth Data](https://www.naturalearthdata.com/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/ned/tz/{lng}/{lat}", 
    tag = "TZ", 
    params(("lng" = f32, Path, description = "The longitude."), ("lat" = f32, Path, description = "The latitude.")), 
    responses(
        (status = 200, description = "List all found timezones successfully.", body = Vec<NedTimezoneResponse1>),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No timezone results: location likely resides on a boundary."),
    )
)]
#[instrument]
async fn timezone_ned(Path((lng, lat)): Path<(Float, Float)>, if_modified_since: IfModifiedSince) -> WebResult<LookupResponse<Vec<NedTimezoneResponse1>>> {
    timezone_ned_v1(Path((lng, lat)), if_modified_since).await
}

/// Gets time zone information from the NED dataset.
///
/// Returns the time zone information for the given `(lng,lat)` from the [Natural Earth Data](https://www.naturalearthdata.com/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/v1/ned/tz/{lng}/{lat}", 
    tag = "TZv1", 
    params(("lng" = f32, Path, description = "The longitude."), ("lat" = f32, Path, description = "The latitude.")), 
    responses(
        (status = 200, description = "List all found timezones successfully.", body = Vec<NedTimezoneResponse1>),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No timezone results: location likely resides on a boundary."),
    )
)]
#[instrument]
async fn timezone_ned_v1(Path((lng, lat)): Path<(Float, Float)>, if_modified_since: IfModifiedSince) -> WebResult<LookupResponse<Vec<NedTimezoneResponse1>>> {
    if if_modified_since.as_str() == get_last_modified_time() {
        log::warn!("Not modified.");
        return Ok(LookupResponse::NotModified);
    }

    let tzs = NedTimezone::lookup(lng, lat).into_iter().map(|tz| tz.into()).collect::<Vec<_>>();

    Ok(LookupResponse::Ok(Json(tzs)))
}

/// Gets time zone information from the OSM dataset.
///
/// Returns the time zone information for the given `(lng,lat)` from the [OpenStreetMap](https://www.openstreetmap.org/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/osm/tz/{lng}/{lat}", 
    tag = "TZ", 
    params(("lng" = f32, Path, description = "The longitude."), ("lat" = f32, Path, description = "The latitude.")), 
    responses(
        (status = 200, description = "List all found timezones successfully.", body = Vec<OsmTimezoneResponse1>),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No timezone results: location likely resides on a boundary."),
    )
)]
#[instrument]
async fn timezone_osm(Path((lng, lat)): Path<(Float, Float)>) -> WebResult<LookupResponse<Vec<OsmTimezoneResponse1>>> {
    timezone_osm_v1(Path((lng, lat))).await
}

/// Gets time zone information from the OSM dataset.
///
/// Returns the time zone information for the given `(lng,lat)` from the [OpenStreetMap](https://www.openstreetmap.org/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/v1/osm/tz/{lng}/{lat}", 
    tag = "TZv1", 
    params(("lng" = f32, Path, description = "The longitude."), ("lat" = f32, Path, description = "The latitude.")), 
    responses(
        (status = 200, description = "List all found timezones successfully.", body = Vec<OsmTimezoneResponse1>),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No timezone results: location likely resides on a boundary."),
    )
)]
#[instrument]
async fn timezone_osm_v1(Path((lng, lat)): Path<(Float, Float)>) -> WebResult<LookupResponse<Vec<OsmTimezoneResponse1>>> {
    let tzs = OsmTimezone::lookup(lng, lat).into_iter().map(|tz| tz.into()).collect::<Vec<_>>();

    Ok(LookupResponse::Ok(Json(tzs)))
}

/// Gets the admin information from the OSM dataset.
///
/// Returns the admin information for the given `(lng,lat)` from the [OpenStreetMap](https://www.openstreetmap.org/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/osm/admin/{lng}/{lat}", 
    tag = "Admin", 
    params(("lng" = f32, Path, description = "The longitude."), ("lat" = f32, Path, description = "The latitude.")), 
    responses(
        (status = 200, description = "List all found administrative districts successfully.", body = Vec<OsmAdminResponse1>),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No results: location likely resides on a boundary."),
    )
)]
#[instrument]
async fn admin_osm(Path((lng, lat)): Path<(Float, Float)>) -> WebResult<LookupResponse<Vec<OsmAdminResponse1>>> {
    admin_osm_v1(Path((lng, lat))).await
}

/// Gets the admin information from the OSM dataset.
///
/// Returns the admin information for the given `(lng,lat)` from the [OpenStreetMap](https://www.openstreetmap.org/) dataset.
///
/// This API endpoint is provided under the same [license](https://github.com/twitchax/rtz/blob/main/LICENSE) as the
/// [project](https://github.com/twitchax/rtz) itself.  It is provided as-is, with no warranty (as of today).
#[utoipa::path(
    get,
    context_path = "/api", 
    path = "/v1/osm/admin/{lng}/{lat}", 
    tag = "Adminv1", 
    params(("lng" = f32, Path, description = "The longitude."), ("lat" = f32, Path, description = "The latitude.")), 
    responses(
        (status = 200, description = "List all found administrative districts successfully.", body = Vec<OsmAdminResponse1>),
        (status = 304, description = "Not modified."),
        (status = 404, description = "No results: location likely resides on a boundary."),
    )
)]
#[instrument]
async fn admin_osm_v1(Path((lng, lat)): Path<(Float, Float)>) -> WebResult<LookupResponse<Vec<OsmAdminResponse1>>> {
    let admins = OsmAdmin::lookup(lng, lat).into_iter().map(|a| a.into()).collect::<Vec<_>>();

    Ok(LookupResponse::Ok(Json(admins)))
}
