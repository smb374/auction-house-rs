use std::sync::Arc;

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::TimeDelta;
use serde_dynamo::{from_item, from_items};
use ulid::Ulid;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::ITEM_TABLE,
    errors::HandlerError,
    models::{
        auth::ClaimOwned,
        item::{CheckItemExiprationResponse, Item, ItemState},
        user::UserType,
    },
    routes::check_user,
    state::AppState,
};

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_item))
        .routes(routes!(get_active_items))
        .routes(routes!(check_item_expiration))
        .routes(routes!(get_recently_sold))
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
    security(
        ("http-jwt" = []),
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

/// Check Expiration Status of the Item
#[utoipa::path(
    post,
    path = "/{sellerId}/{itemId}/check-expired",
    tag = "Item",
    params(
        ("sellerId" = String, Path, description = "Seller of the item"),
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    responses(
        (status = OK, description = "Returns specified item", body = CheckItemExiprationResponse),
        (status = BAD_REQUEST, description = "Item not published", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn check_item_expiration(
    State(state): State<Arc<AppState>>,
    Path((seller_id, item_id)): Path<(String, Ulid)>,
) -> Result<Json<CheckItemExiprationResponse>, HandlerError> {
    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .get_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(seller_id))
        .key("id", AttributeValue::S(item_id.to_string()))
        .send()
        .await?;

    let item = get_item_resp.item.ok_or(HandlerError::not_found())?;

    let result: Item = from_item(item)?;

    let current_time = chrono::Local::now().timestamp_millis() as u64;
    match result.end_date {
        Some(edate) => Ok(Json(CheckItemExiprationResponse {
            seller_id: result.seller_id,
            id: result.id,
            is_expired: current_time > edate,
        })),
        None => Err(HandlerError::HandlerError(
            StatusCode::BAD_REQUEST,
            "Item hasn't been published.".to_string(),
        )),
    }
}

/// Get recently sold items
#[utoipa::path(
    get,
    path = "/recently-sold",
    tag = "Item",
    responses(
        (status = OK, description = "Return recently sold items", body = Vec<Item>),
        (status = FORBIDDEN, description = "Not a buyer", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
    security(
        ("http-jwt" = []),
    ),
)]
async fn get_recently_sold(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Item>>, HandlerError> {
    check_user(claim.as_claim(), UserType::Buyer)?;

    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .scan()
        .table_name(ITEM_TABLE)
        .filter_expression("state = :archived AND soldTime <> :null")
        .expression_attribute_values(":archived", ItemState::Archived.into())
        .expression_attribute_values(":null", AttributeValue::Null(true))
        .send()
        .await?;

    let result: Vec<Item> = from_items(get_item_resp.items().to_vec())?;

    let now = chrono::Local::now().timestamp_millis();
    let delta = TimeDelta::days(1).num_milliseconds();

    let filtered = result
        .into_iter()
        .filter(|item| {
            item.sold_time.map_or(false, |t| {
                let diff = now - t as i64;
                diff > 0 && diff < delta
            })
        })
        .collect();

    Ok(Json(filtered))
}
