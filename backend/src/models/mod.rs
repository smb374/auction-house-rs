use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod auth;
pub mod bid;
pub mod item;
pub mod user;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlainSuccessResponse {
    pub status: u16,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub status: u16,
    pub inner_status: Option<u16>,
    pub message: String,
}

impl IntoResponse for PlainSuccessResponse {
    fn into_response(self) -> Response {
        let code = StatusCode::from_u16(self.status).unwrap();
        let body = Json(self);

        (code, body).into_response()
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let code = StatusCode::from_u16(self.status).unwrap();
        let body = Json(self);

        (code, body).into_response()
    }
}

pub type PlainResult = Result<PlainSuccessResponse, ErrorResponse>;
pub type GeneralResult<T> = Result<T, ErrorResponse>;
