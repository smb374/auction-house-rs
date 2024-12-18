use aws_sdk_dynamodb::{
    error::SdkError as DynamoSdkError,
    operation::{
        delete_item::DeleteItemError, get_item::GetItemError, put_item::PutItemError,
        query::QueryError, transact_write_items::TransactWriteItemsError,
        update_item::UpdateItemError,
    },
};
use axum::{
    http::{self, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{PartialSchema, ToSchema};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    status: u16,
    inner_status: Option<u16>,
    message: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let code = StatusCode::from_u16(self.status).unwrap();
        let body = Json(self);

        (code, body).into_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("DynamoDB Error: GetItem: {0}")]
    DynamoDBGetError(#[from] DynamoSdkError<GetItemError>),
    #[error("DynamoDB Error: PutItem: {0}")]
    DynamoDBPutError(#[from] DynamoSdkError<PutItemError>),
    #[error("DynamoDB Error: Query: {0}")]
    DynamoDBQueryError(#[from] DynamoSdkError<QueryError>),
    #[error("DynamoDB Error: DeleteItem: {0}")]
    DynamoDBDeleteError(#[from] DynamoSdkError<DeleteItemError>),
    #[error("DynamoDB Error: UpdateItem: {0}")]
    DynamoDBUpdateError(#[from] DynamoSdkError<UpdateItemError>),
    #[error("DynamoDB Error: TransactWriteItems: {0}")]
    DynamoDBTransactWriteItemsError(#[from] DynamoSdkError<TransactWriteItemsError>),
    #[error("Failed to build transaction: {0}")]
    TransactionBuildError(#[from] aws_sdk_dynamodb::error::BuildError),
    #[error("JWT operation failed: {0}")]
    JWTError(#[from] jsonwebtoken::errors::Error),
    #[error("PasswordHash error: {0}")]
    PasswordHashError(#[from] scrypt::password_hash::Error),
    #[error("SerdeDynamo failed to process DynamoDB data: {0}")]
    SerdeDynamoError(#[from] serde_dynamo::Error),
    #[error("HTTP library error: {0}")]
    HttpError(#[from] http::Error),
    #[error("Handler failed with status {0}: {1}")]
    HandlerError(StatusCode, String),
}

impl From<HandlerError> for ErrorResponse {
    fn from(value: HandlerError) -> Self {
        Self {
            status: if let &HandlerError::HandlerError(s, _) = &value {
                s.as_u16()
            } else {
                StatusCode::INTERNAL_SERVER_ERROR.as_u16()
            },
            inner_status: match &value {
                HandlerError::DynamoDBGetError(e) => e.raw_response().map(|r| r.status().as_u16()),
                HandlerError::DynamoDBPutError(e) => e.raw_response().map(|r| r.status().as_u16()),
                HandlerError::DynamoDBQueryError(e) => {
                    e.raw_response().map(|r| r.status().as_u16())
                }
                HandlerError::DynamoDBDeleteError(e) => {
                    e.raw_response().map(|r| r.status().as_u16())
                }
                HandlerError::DynamoDBUpdateError(e) => {
                    e.raw_response().map(|r| r.status().as_u16())
                }
                _ => None,
            },
            message: value.to_string(),
        }
    }
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> axum::response::Response {
        ErrorResponse::from(self).into_response()
    }
}

impl PartialSchema for HandlerError {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        ErrorResponse::schema()
    }
}

impl ToSchema for HandlerError {
    fn schemas(
        schemas: &mut Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) {
        <ErrorResponse as ToSchema>::schemas(schemas);
    }
}

impl HandlerError {
    pub fn not_found() -> Self {
        Self::HandlerError(StatusCode::NOT_FOUND, "Item not found".to_string())
    }
}
