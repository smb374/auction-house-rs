mod auth;
mod seller;

use axum::{
    body::{Body, HttpBody},
    response::Response,
};
use lambda_http::Error;
use serde::de::DeserializeOwned;

async fn parse_resp<T: DeserializeOwned>(resp: Response<Body>) -> Result<T, Error> {
    let body = resp.into_body();
    let limit = body.size_hint().upper().unwrap_or(u64::MAX) as usize;
    let data = axum::body::to_bytes(body, limit).await?;
    let res: T = serde_json::from_slice(&data)?;

    Ok(res)
}
