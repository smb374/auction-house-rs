use std::sync::Arc;

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Path, State},
    Json,
};
use serde_dynamo::{from_item, from_items};
use ulid::Ulid;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::ITEM_TABLE,
    errors::HandlerError,
    models::item::{Item, ItemState},
    state::AppState,
};

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_item))
        .routes(routes!(get_active_items))
}

// Get Item
/// Get seller's item by sellerId and itemId.
#[utoipa::path(
    get,
    path = "/{sellerId}/{itemId}",
    tag = "Item",
    params(
        ("sellerId" = String, Path, description = "Seller of the item"),
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    responses(
        (status = OK, description = "Returns specified item", body = Item),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn get_item(
    State(state): State<Arc<AppState>>,
    Path((seller_id, item_id)): Path<(String, Ulid)>,
) -> Result<Json<Item>, HandlerError> {
    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .get_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(seller_id))
        .key("id", AttributeValue::S(item_id.to_string()))
        .send()
        .await?;

    let item = get_item_resp.item.ok_or(HandlerError::not_found())?;

    let result = from_item(item)?;

    Ok(Json(result))
}

// Get Item
/// Get all active items
#[utoipa::path(
    get,
    path = "/active",
    tag = "Item",
    responses(
        (status = OK, description = "Return active items", body = Vec<Item>),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn get_active_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Item>>, HandlerError> {
    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .scan()
        .table_name(ITEM_TABLE)
        .filter_expression("state = :active")
        .expression_attribute_values(":active", ItemState::Active.into())
        .send()
        .await?;

    let result = from_items(get_item_resp.items().to_vec())?;

    Ok(Json(result))
}
