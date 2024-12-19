use std::sync::Arc;

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{extract::State, Extension, Json};
use serde_dynamo::to_attribute_value;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::BUYER_TABLE,
    errors::HandlerError,
    models::{auth::ClaimOwned, user::UserType, AddFundRequest},
    state::AppState,
};

use super::check_user;

pub fn route() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(buyer_add_fund))
}

/// Add fund to buyer
#[utoipa::path(
    post,
    path = "/add-fund",
    tag = "Buyer",
    request_body = AddFundRequest,
    responses(
        (status = OK, description = "Add fund success"),
        (status = FORBIDDEN, description = "Not a buyer", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
    security(
        ("http-jwt" = []),
    ),
)]
async fn buyer_add_fund(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddFundRequest>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim(), UserType::Buyer)?;

    let client = Client::new(&state.aws_config);

    client
        .update_item()
        .table_name(BUYER_TABLE)
        .key("id", AttributeValue::S(claim.id.clone()))
        .update_expression("SET fund = fund + :amount")
        .expression_attribute_values(":amount", to_attribute_value(payload.amount)?)
        .send()
        .await?;

    Ok(())
}
