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
    http::{Request, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use lambda_http::{run, tracing, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use state::AppState;
use tower_http::trace::TraceLayer;

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

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    tracing::info!("API Handler Start!!!");

    let state = AppState::new().await?;

    let trace_layer =
        TraceLayer::new_for_http().on_request(|req: &Request<Body>, _: &tracing::Span| {
            let path = req.uri().path();
            tracing::info!("Got request with path: {}", path);
        });

    let app = Router::new()
        .route("/v1/", get(root))
        .route("/v1/utc", get(get_utc))
        .route("/v1/health", get(health_check))
        .route("/v1/register", post(routes::auth::register))
        .layer(trace_layer)
        .with_state(Arc::new(state));

    run(app).await
}
