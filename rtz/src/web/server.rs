use std::sync::Arc;

use axum::{extract::Path, routing::get, Json, Router};
use rtz_core::{
    base::types::{Float, Void},
    geo::{
        admin::osm::OsmAdmin,
        tz::{ned::NedTimezone, osm::OsmTimezone},
    },
};
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
    types::{get_last_modified_time, AppState, IfModifiedSince, WebResult},
};

/// Starts the web server.
pub async fn start(config: &Config) -> Void {
    let app = create_axum_app(config);

    let bind_address = format!("{}:{}", config.bind_address, config.port);
    axum::Server::bind(&bind_address.parse().unwrap()).serve(app.into_make_service()).await.unwrap();

    Ok(())
}

pub fn create_axum_app(config: &Config) -> Router {
    let state = AppState { config: Arc::new(config.clone()) };

    let api_router = Router::new()
        .route("/ned/tz/:lng/:lat", get(timezone_ned))
        .route("/v1/ned/tz/:lng/:lat", get(timezone_ned_v1))
        .route("/osm/tz/:lng/:lat", get(timezone_osm))
        .route("/v1/osm/tz/:lng/:lat", get(timezone_osm_v1))
        .route("/osm/admin/:lng/:lat", get(admin_osm))
        .route("/v1/osm/admin/:lng/:lat", get(admin_osm_v1));

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        .nest("/api", api_router)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    app
}

#[derive(OpenApi)]
#[openapi(
    paths(timezone_ned, timezone_ned_v1, timezone_osm, timezone_osm_v1, admin_osm, admin_osm_v1),
    components(schemas(NedTimezoneResponse1, OsmTimezoneResponse1, OsmAdminResponse1))
)]
struct ApiDoc;

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
async fn admin_osm_v1(Path((lng, lat)): Path<(Float, Float)>) -> WebResult<LookupResponse<Vec<OsmAdminResponse1>>> {
    let admins = OsmAdmin::lookup(lng, lat).into_iter().map(|a| a.into()).collect::<Vec<_>>();

    Ok(LookupResponse::Ok(Json(admins)))
}
