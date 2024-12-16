mod constants;
mod middlewares;
mod models;
mod routes;
mod state;
mod utils;

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use lambda_http::{run, tracing, Error};
use models::GeneralResult;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use state::AppState;
use tower_http::trace::TraceLayer;
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
struct Resp {
    utc: u64,
}

async fn get_utc() -> Json<Resp> {
    let unixtime = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    Json(Resp {
        utc: unixtime.as_secs(),
    })
}

async fn root() -> Json<Value> {
    Json(json!({ "msg": "I am GET /" }))
}

/// Example on how to return status codes and data from an Axum function
async fn health_check() -> (StatusCode, String) {
    let health = true;
    match health {
        true => (StatusCode::OK, "Healthy!".to_string()),
        false => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Not healthy!".to_string(),
        ),
    }
}

async fn serve_openapi(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        (
            [(header::CONTENT_TYPE, "application/yaml")],
            state.oapi.clone(),
        ),
    )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    tracing::info!("API Handler Start!!!");

    let trace_layer =
        TraceLayer::new_for_http().on_request(|req: &Request<Body>, _: &tracing::Span| {
            let path = req.uri().path();
            tracing::info!("Got request with path: {}", path);
        });

    let (router, oapi) = OpenApiRouter::new()
        .route("/v1/", get(root))
        .route("/v1/utc", get(get_utc))
        .route("/v1/health", get(health_check))
        .merge(OpenApiRouter::new().routes(routes!(routes::auth::register)))
        .merge(OpenApiRouter::new().routes(routes!(routes::auth::login_challenge)))
        .merge(OpenApiRouter::new().routes(routes!(routes::auth::login)))
        // .route("/v1/register", post(routes::auth::register))
        // .route("/v1/login/challenge", post(routes::auth::login_challenge))
        // .route("/v1/login", post(routes::auth::login_challenge_response))
        .layer(trace_layer)
        .split_for_parts();

    let yaml = oapi.to_yaml()?;
    let state = AppState::new(yaml).await?;

    let service = router
        .route("/v1/openapi", get(serve_openapi))
        .with_state(Arc::new(state));

    run(service).await
}
