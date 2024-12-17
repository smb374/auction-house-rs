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

impl ErrorResponse {
    pub fn new<S: Into<String>>(status: StatusCode, message: S) -> Self {
        Self {
            status: status.as_u16(),
            inner_status: None,
            message: message.into(),
        }
    }

    pub fn with_inner_status<S: Into<String>>(inner_status: Option<u16>, message: S) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status,
            message: message.into(),
        }
    }

    pub fn generic<E: std::error::Error, S: Into<String>>(message: S, e: E) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{}: {}", message.into(), e),
        )
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "Not Found")
    }
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
