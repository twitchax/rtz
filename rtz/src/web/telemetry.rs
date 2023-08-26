use std::panic;

use axum::{middleware::Next, response::Response, RequestPartsExt, extract::MatchedPath};
use hyper::{Request, body::{Bytes, HttpBody}, Body};
use opentelemetry::{sdk::{trace::Config, self}, KeyValue};
use reqwest::Client;
use tracing::{Span, Instrument, Level, level_filters};
use tracing_subscriber::{Registry, prelude::__tracing_subscriber_SubscriberExt};

use super::types::WebError;

// Methods.

/// Initializes the application insights [`ConcreteTracer`].
fn init_tracer(key: &str) {
    let config = Config::default().with_resource(sdk::Resource::new(vec![KeyValue::new("service.namespace", "rtz"), KeyValue::new("service.name", "server")]));

    let tracer = opentelemetry_application_insights::new_pipeline_from_connection_string(key)
        .unwrap()
        .with_client(Client::new())
        .with_trace_config(config)
        .install_batch(opentelemetry::runtime::Tokio);

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry).with(level_filters::LevelFilter::INFO);
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

/// Initializes the global telemetry tracer with the given application insights key.
pub fn init(key: Option<&str>) {
    let Some(key) = key else {
        return;
    };

    init_tracer(key);

    let default_panic = panic::take_hook();

    panic::set_hook(Box::new(move |p| {
        let payload_string = format!("{:?}", p.payload().downcast_ref::<&str>());
        let location_string = p.location().map(|l| format!("{}", l)).unwrap_or_else(|| "unknown".to_owned());

        // This doesn't work because this macro prescribes the name without allowing it to be overriden.
        tracing::event!(Level::ERROR, event.name = "exception", "exception.type" = "PANIC", exception.message = payload_string, exception.stacktrace = location_string);

        default_panic(p);
    }));
}

/// The axum layer for request telemetry.
pub async fn telemetry_fn<B>(
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let client_ip = client_ip
        .split(',')
        .next()
        .unwrap_or("unknown");
    
    let (mut parts, body) = request.into_parts();
    let route = parts.extract::<MatchedPath>().await.map(|m| m.as_str().to_owned()).unwrap_or_else(|_| "unknown".to_owned());
    let request = Request::from_parts(parts, body);

    let span = tracing::info_span!(
        "request",
        otel.kind = "server",
        http.method = method.as_str(),
        http.url = uri.as_str(),
        http.client_ip = client_ip,
        http.route = route.as_str(),
        otel.status_code = tracing::field::Empty,
        otel.status_description = tracing::field::Empty,
        http.response.status_code = tracing::field::Empty,
    );

    async move {
        /////////////////////////////////
        let response = next.run(request).await;
        /////////////////////////////////
        
        let status = response.status();
        let (response, otel_status, otel_status_description) = if status.is_success() {
            (response, "OK", format!(r#"{{ "status": {} }}"#, status.as_u16()))
        } else {
            // Breakup the response into parts.
            let (parts, body) = response.into_parts();

            // Get the body bytes.
            let body_bytes = hyper::body::to_bytes(body).await.unwrap_or(Bytes::new());

            // Deserialize the error.
            let error: WebError = serde_json::from_slice(&body_bytes).unwrap_or_else(|_| WebError {
                status: status.as_u16(),
                message: "UNKNOWN".to_string(),
                backtrace: None,
            });

            // Get the stringified error.
            let error_string = serde_json::to_string_pretty(&error).unwrap();

            // Recreate the body.
            let body = Body::from(body_bytes).boxed_unsync().map_err(axum::Error::new).boxed_unsync();

            let response = Response::from_parts(parts, body);

            (response, "ERROR", error_string)
        };
        
        let span = Span::current().entered();

        span.record("otel.status_code", otel_status);
        span.record("otel.status_description", otel_status_description);
        span.record("http.response.status_code", status.as_u16());

        response
    }.instrument(span).await
}