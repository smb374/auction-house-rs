use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{Algorithm, Validation};

use crate::{
    models::{auth::ClaimOwned, ErrorResponse, GeneralResult},
    state::AppState,
};

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> GeneralResult<Response<Body>> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .map(|h| h.to_str());
    let header = match auth_header {
        Some(h) => h.map_err(|e| ErrorResponse {
            status: StatusCode::FORBIDDEN.as_u16(),
            inner_status: None,
            message: format!("Failed to down cast header value to string: {}", e),
        })?,
        None => {
            return Err(ErrorResponse {
                status: StatusCode::FORBIDDEN.as_u16(),
                inner_status: None,
                message: "Invalid authorization header.".to_string(),
            })
        }
    };
    // token should be "Bearer ..."
    let mut it = header.split_whitespace();
    let (_, token_str) = (it.next(), it.next());
    let token = token_str.ok_or(ErrorResponse {
        status: StatusCode::FORBIDDEN.as_u16(),
        inner_status: None,
        message: "Empty token value".to_string(),
    })?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["auction-house-rs"]);
    let data =
        jsonwebtoken::decode::<ClaimOwned>(token, &state.jwt.1, &validation).map_err(|e| {
            ErrorResponse {
                status: StatusCode::FORBIDDEN.as_u16(),
                inner_status: None,
                message: format!("Failed to decode JWT token: {}", e),
            }
        })?;
    req.extensions_mut().insert(data.claims);

    Ok(next.run(req).await)
}
