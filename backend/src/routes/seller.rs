use std::{collections::HashMap, sync::Arc};

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    Extension,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::ITEM_TABLE,
    models::{
        auth::{Claim, ClaimOwned},
        item::{AddItemRequest, Item, ItemRef, ItemState, UpdateItemRequest},
        user::UserType,
        ErrorResponse, GeneralResult,
    },
    state::AppState,
};

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(seller_get_owned_items, seller_add_item))
        .routes(routes!(
            seller_get_item_by_id,
            seller_delete_item_by_id,
            seller_update_item_by_id
        ))
}

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
async fn seller_get_owned_items(
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

/// Add an item under a seller.
#[utoipa::path(
    put,
    path = "/v1/seller/item",
    tag = "Seller",
    request_body = AddItemRequest,
    responses(
        (status = OK, description = "Add item success", body = ItemRef),
        (status = BAD_REQUEST, description = "Bad add request", body = ErrorResponse),
        (status = FORBIDDEN, description = "Not a seller", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn seller_add_item(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddItemRequest>,
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

/// Get seller's item by itemId.
#[utoipa::path(
    get,
    path = "/v1/seller/item/{itemId}",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    responses(
        (status = OK, description = "Register Success", body = Item),
        (status = FORBIDDEN, description = "Not a seller", body = ErrorResponse),
        (status = NOT_FOUND, description = "Item not found", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn seller_get_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
) -> GeneralResult<Json<Item>> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .get_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id))
        .send()
        .await
        .map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: e.raw_response().map(|r| r.status().as_u16()),
            message: format!("Failed to query seller items: {}", e),
        })?;

    let item = get_item_resp.item.ok_or(ErrorResponse {
        status: StatusCode::NOT_FOUND.as_u16(),
        inner_status: None,
        message: "Item not found.".to_string(),
    })?;

    let result = serde_dynamo::from_item(item).map_err(|e| ErrorResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        inner_status: None,
        message: format!("Failed to deserialize query result for seller items: {}", e),
    })?;

    Ok(Json(result))
}

/// Delete seller's item by itemId.
#[utoipa::path(
    delete,
    path = "/v1/seller/item/{itemId}",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    responses(
        (status = OK, description = "Register Success", body = Item),
        (status = FORBIDDEN, description = "Not a seller", body = ErrorResponse),
        (status = NOT_FOUND, description = "Item not found", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn seller_delete_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
) -> GeneralResult<()> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let delete_item_resp = client
        .delete_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id))
        .condition_expression("itemState = :val")
        .expression_attribute_values(":val", AttributeValue::S(ItemState::InActive.to_string()))
        .send()
        .await
        .map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: e.raw_response().map(|r| r.status().as_u16()),
            message: format!("Failed to query seller items: {}", e),
        })?;

    if delete_item_resp.attributes().is_none() {
        Err(ErrorResponse {
            status: StatusCode::NOT_FOUND.as_u16(),
            inner_status: None,
            message: "Item not found.".to_string(),
        })
    } else {
        Ok(())
    }
}

/// Update seller's item by itemId.
#[utoipa::path(
    post,
    path = "/v1/seller/item/{itemId}",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    request_body = UpdateItemRequest,
    responses(
        (status = OK, description = "Add item success", body = ItemRef),
        (status = BAD_REQUEST, description = "Bad update request", body = ErrorResponse),
        (status = FORBIDDEN, description = "Not a seller", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn seller_update_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
    Json(payload): Json<UpdateItemRequest>,
) -> GeneralResult<()> {
    check_user(claim.as_claim())?;

    if payload == UpdateItemRequest::default() {
        return Err(ErrorResponse {
            status: StatusCode::BAD_REQUEST.as_u16(),
            inner_status: None,
            message: "Update request must have at least 1 field to update.".to_string(),
        });
    }

    let mut update_expr: Vec<&str> = Vec::new();
    let mut eavs: HashMap<String, AttributeValue> = HashMap::new();

    let client = Client::new(&state.aws_config);
    let mut update_item_cmd = client
        .update_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id))
        .condition_expression("itemState = :state");

    eavs.insert(
        ":state".to_string(),
        AttributeValue::S(ItemState::InActive.to_string()),
    );

    if let Some(name) = payload.name {
        update_expr.push("name = :name");
        eavs.insert(":name".to_string(), AttributeValue::S(name));
    }

    if let Some(description) = payload.description {
        update_expr.push("description = :description");
        eavs.insert(":description".to_string(), AttributeValue::S(description));
    }

    if let Some(init_price) = payload.init_price {
        update_expr.push("init_price = :init_price");
        eavs.insert(
            ":init_price".to_string(),
            AttributeValue::N(format!("{}", init_price)),
        );
    }

    if let Some(auction_length) = payload.auction_length {
        update_expr.push("auction_length = :auction_length");
        eavs.insert(
            ":auction_length".to_string(),
            AttributeValue::N(format!("{}", auction_length)),
        );
    }

    if let Some(images) = payload.images {
        update_expr.push("images = :images");
        eavs.insert(":images".to_string(), AttributeValue::Ss(images));
    }

    update_item_cmd = update_item_cmd
        .update_expression(format!("SET {}", update_expr.join(", ")))
        .set_expression_attribute_values(Some(eavs));

    update_item_cmd.send().await.map_err(|e| ErrorResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        inner_status: e.raw_response().map(|r| r.status().as_u16()),
        message: format!("Failed to update item: {}", e),
    })?;

    Ok(())
}
