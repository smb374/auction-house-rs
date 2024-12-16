use std::sync::Arc;

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    Extension,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::ITEM_TABLE,
    models::{
        auth::{Claim, ClaimOwned},
        item::{Item, ItemRef, PutItemRequest},
        user::UserType,
        ErrorResponse, GeneralResult,
    },
    state::AppState,
};

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(get_seller_items, add_item))
}

// async fn get_seller(client: &Client, id: &str) -> GeneralResult<Seller> {
//     todo!()
// }

fn check_user(claim: Claim) -> GeneralResult<()> {
    if claim.user_type != UserType::Seller {
        return Err(ErrorResponse {
            status: StatusCode::FORBIDDEN.as_u16(),
            inner_status: None,
            message: "Only seller can use this route.".to_string(),
        });
    }
    Ok(())
}

/// Get all of seller's items.
#[utoipa::path(
    get,
    path = "/v1/seller/item",
    tag = "Seller",
    responses(
        (status = OK, description = "Register Success", body = Vec<Item>),
        (status = FORBIDDEN, description = "Not a seller", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn get_seller_items(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
) -> GeneralResult<Json<Vec<Item>>> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let query_item_resp = client
        .query()
        .table_name(ITEM_TABLE)
        .key_condition_expression("sellerId = :sid")
        .expression_attribute_values(":sid", AttributeValue::S(claim.id.clone()))
        .send()
        .await
        .map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: e.raw_response().map(|r| r.status().as_u16()),
            message: format!("Failed to query seller items: {}", e),
        })?;
    let items: Vec<Item> =
        serde_dynamo::from_items(query_item_resp.items().to_vec()).map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: None,
            message: format!("Failed to deserialize query result for seller items: {}", e),
        })?;

    Ok(Json(items))
}

/// Get all of seller's items.
#[utoipa::path(
    put,
    path = "/v1/seller/item",
    tag = "Seller",
    responses(
        (status = OK, description = "Add item success", body = ItemRef),
        (status = BAD_REQUEST, description = "Bad add request", body = ErrorResponse),
        (status = FORBIDDEN, description = "Not a seller", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn add_item(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PutItemRequest>,
) -> GeneralResult<Json<ItemRef>> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let new_item = Item::new_from_request(claim.id.clone(), payload);
    let iref = ItemRef::from(&new_item);
    let item = serde_dynamo::to_item(new_item).map_err(|e| ErrorResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        inner_status: None,
        message: format!("Failed to serialize item: {}", e),
    })?;

    client
        .put_item()
        .table_name(ITEM_TABLE)
        .set_item(Some(item))
        .send()
        .await
        .map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: e.raw_response().map(|r| r.status().as_u16()),
            message: format!("Failed to put item: {}", e),
        })?;

    Ok(Json(iref))
}
