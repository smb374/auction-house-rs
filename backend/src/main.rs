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
    http::{header, StatusCode},
    middleware,
    response::{Json, Response},
    routing::get,
    Extension,
};
use lambda_http::{run, tracing, Error};
use models::{ErrorResponse, GeneralResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use state::AppState;
use tower_http::compression::CompressionLayer;
use utoipa::openapi::OpenApi;
use utoipa_axum::router::OpenApiRouter;

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

async fn serve_openapi(Extension(oapi): Extension<OpenApi>) -> GeneralResult<Response<String>> {
    let yaml = oapi.to_yaml().map_err(|e| ErrorResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        inner_status: None,
        message: format!("Failed to serialize OpenAPI spec: {}", e),
    })?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/yaml")
        .body(yaml)
        .map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: None,
            message: format!("Failed to construct response: {}", e),
        })
}

async fn ping() -> String {
    "PONG".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    tracing::info!("API Handler Start!!!");

    let state = Arc::new(AppState::new().await?);

    let plain_router = OpenApiRouter::new()
        .route("/v1/", get(root))
        .route("/v1/utc", get(get_utc))
        .route("/v1/health", get(health_check))
        .merge(routes::auth::router())
        .with_state(state.clone());

    let auth_router =
        OpenApiRouter::new()
            .route("/v1/ping", get(ping))
            .layer(middleware::from_fn_with_state(
                state,
                middlewares::auth::auth_middleware,
            ));

    let (router, oapi) = OpenApiRouter::new()
        .merge(plain_router)
        .merge(auth_router)
        .layer(CompressionLayer::new().zstd(true))
        .layer(middleware::from_fn(middlewares::trace_client))
        .split_for_parts();

    let service = router
        .route("/v1/openapi", get(serve_openapi))
        .layer(Extension(oapi));

    run(service).await
}
