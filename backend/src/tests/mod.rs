mod auth;
mod item;
mod seller;

use axum::{
    body::{Body, HttpBody},
    extract::Request,
    response::Response,
};
use lambda_http::Error;
use serde::{de::DeserializeOwned, Serialize};

async fn parse_resp<T: DeserializeOwned>(resp: Response<Body>) -> Result<T, Error> {
    let body = resp.into_body();
    let limit = body.size_hint().upper().unwrap_or(u64::MAX) as usize;
    let data = axum::body::to_bytes(body, limit).await?;
    let res: T = serde_json::from_slice(&data)?;

    Ok(res)
}

fn build_request<T: Serialize>(
    method: &str,
    uri: &str,
    token: &str,
    body: Option<T>,
) -> Result<Request<Body>, Error> {
    let req = match body {
        Some(v) => {
            let content = serde_json::to_string(&v)?;
            Request::builder()
                .method(method)
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::new(content))
        }
        None => Request::builder()
            .method(method)
            .uri(uri)
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty()),
    }?;
    Ok(req)
}
