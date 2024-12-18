use std::{collections::HashMap, sync::Arc};

use aws_sdk_dynamodb::{
    types::{AttributeValue, Put, TransactWriteItem, Update},
    Client,
};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    Extension,
};
use serde_dynamo::{from_item, from_items, to_attribute_value, to_item};
use ulid::Ulid;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::{BID_TABLE, BUYER_TABLE, ITEM_TABLE, PURCHASE_TABLE, SELLER_TABLE},
    errors::HandlerError,
    models::{
        auth::{Claim, ClaimOwned},
        bid::{Bid, Purchase},
        item::{AddItemRequest, Item, ItemRef, ItemState, UpdateItemRequest},
        user::UserType,
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
        .routes(routes!(seller_publish_item_by_id))
        .routes(routes!(seller_unpublish_item_by_id))
        .routes(routes!(seller_fulfill_item_by_id))
        .routes(routes!(seller_archive_item_by_id))
}

fn check_user(claim: Claim) -> Result<(), HandlerError> {
    if claim.user_type != UserType::Seller {
        return Err(HandlerError::HandlerError(
            StatusCode::FORBIDDEN,
            "Only seller can use this".to_string(),
        ));
    }
    Ok(())
}

// Review Items
/// Get all of seller's items.
#[utoipa::path(
    get,
    path = "/item",
    tag = "Seller",
    responses(
        (status = OK, description = "Returns all seller items", body = Vec<Item>),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_get_owned_items(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Item>>, HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let query_item_resp = client
        .query()
        .table_name(ITEM_TABLE)
        .key_condition_expression("sellerId = :sid")
        .expression_attribute_values(":sid", AttributeValue::S(claim.id.clone()))
        .send()
        .await?;

    let items: Vec<Item> = from_items(query_item_resp.items().to_vec())?;

    Ok(Json(items))
}

// Add Item
/// Add an item under a seller.
#[utoipa::path(
    put,
    path = "/item",
    tag = "Seller",
    request_body = AddItemRequest,
    responses(
        (status = OK, description = "Add item success", body = ItemRef),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_add_item(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddItemRequest>,
) -> Result<Json<ItemRef>, HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let new_item = Item::new_from_request(claim.id.clone(), payload);
    let iref = ItemRef::from(&new_item);
    let item = to_item(new_item)?;

    client
        .put_item()
        .table_name(ITEM_TABLE)
        .set_item(Some(item))
        .send()
        .await?;

    Ok(Json(iref))
}

// Get Item
/// Get seller's item by itemId.
#[utoipa::path(
    get,
    path = "/item/{itemId}",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    responses(
        (status = OK, description = "Returns specified item", body = Item),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_get_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
) -> Result<Json<Item>, HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .get_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id.to_string()))
        .send()
        .await?;

    let item = get_item_resp.item.ok_or(HandlerError::not_found())?;

    let result = from_item(item)?;

    Ok(Json(result))
}

// Remove inactive item
/// Delete seller's item by itemId.
#[utoipa::path(
    delete,
    path = "/item/{itemId}",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to get", format = Ulid),
    ),
    responses(
        (status = OK, description = "Item deleted"),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_delete_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let delete_item_resp = client
        .delete_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id.to_string()))
        .condition_expression("itemState = :val")
        .expression_attribute_values(":val", AttributeValue::S(ItemState::InActive.to_string()))
        .send()
        .await?;

    if delete_item_resp.attributes().is_none() {
        Err(HandlerError::not_found())
    } else {
        Ok(())
    }
}

// Edit item
/// Update seller's item by itemId.
#[utoipa::path(
    post,
    path = "/item/{itemId}",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to update", format = Ulid),
    ),
    request_body = UpdateItemRequest,
    responses(
        (status = OK, description = "Update item success"),
        (status = BAD_REQUEST, description = "Bad update request", body = HandlerError),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_update_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
    Json(payload): Json<UpdateItemRequest>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim())?;

    if payload == UpdateItemRequest::default() {
        return Err(HandlerError::HandlerError(
            StatusCode::BAD_REQUEST,
            "Must have at least 1 field to update.".to_string(),
        ));
    }

    let mut update_expr: Vec<&str> = Vec::new();
    let mut eavs: HashMap<String, AttributeValue> = HashMap::new();

    let client = Client::new(&state.aws_config);
    let mut update_item_cmd = client
        .update_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id.to_string()))
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

    update_item_cmd.send().await?;

    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishSubItem {
    pub auction_length: i64,
    pub state: ItemState,
}

// Publish Item
/// Publish item by itemId.
#[utoipa::path(
    post,
    path = "/item/{itemId}/publish",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to publish", format = Ulid),
    ),
    responses(
        (status = OK, description = "Item delete success"),
        (status = BAD_REQUEST, description = "Bad request", body = HandlerError),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_publish_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .get_item()
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id.to_string()))
        .projection_expression("state, auctionLength")
        .send()
        .await?;

    let item: PublishSubItem = from_item(get_item_resp.item.ok_or(HandlerError::not_found())?)?;

    if item.state != ItemState::InActive {
        return Err(HandlerError::HandlerError(
            StatusCode::BAD_REQUEST,
            "Item need to be inactive".to_string(),
        ));
    }

    let sdate = chrono::Local::now().timestamp_millis();
    let edate = sdate + item.auction_length;

    client
        .update_item()
        .key("sellerId", AttributeValue::S(claim.id))
        .key("id", AttributeValue::S(item_id.to_string()))
        .update_expression("SET state = :state, startDate = :sdate, endDate = :edate")
        .expression_attribute_values(":state", ItemState::Active.into())
        .expression_attribute_values(":sdate", to_attribute_value(sdate)?)
        .expression_attribute_values(":edate", to_attribute_value(edate)?)
        .send()
        .await?;

    Ok(())
}

// UnPublish Item
/// UnPublish item by itemId.
#[utoipa::path(
    post,
    path = "/item/{itemId}/unpublish",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to unpublish", format = Ulid),
    ),
    responses(
        (status = OK, description = "Item unpublish success", body = Item),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_unpublish_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    client
        .update_item()
        .key("sellerId", AttributeValue::S(claim.id))
        .key("id", AttributeValue::S(item_id.to_string()))
        .update_expression("SET state = :state, startDate = :null, endDate = :null")
        .condition_expression("state = :old_state, currentBid = :null, size(pastBids) = :zero")
        .expression_attribute_values(":state", ItemState::InActive.into())
        .expression_attribute_values(":old_state", ItemState::Active.into())
        .expression_attribute_values(":null", AttributeValue::Null(true))
        .expression_attribute_values(":zero", AttributeValue::N("0".to_string()))
        .send()
        .await?;

    Ok(())
}

/// Fulfill item by itemId.
#[utoipa::path(
    post,
    path = "/item/{itemId}/fulfill",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to fulfill", format = Ulid),
    ),
    responses(
        (status = OK, description = "Item fulfill success"),
        (status = BAD_REQUEST, description = "Item cannot be fulfilled yet", body = HandlerError),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_fulfill_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    let get_item_resp = client
        .get_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(claim.id.clone()))
        .key("id", AttributeValue::S(item_id.to_string()))
        .send()
        .await?;
    let db_item = get_item_resp.item.ok_or(HandlerError::not_found())?;
    let item: Item = from_item(db_item)?;
    if item.state != ItemState::Completed || item.current_bid.is_none() {
        return Err(HandlerError::HandlerError(
            StatusCode::BAD_REQUEST,
            "This item cannot be fulfilled.".to_string(),
        ));
    }

    let curr_bid_ref = item.current_bid.as_ref().unwrap();
    let get_bid_resp = client
        .get_item()
        .table_name(BID_TABLE)
        .key("buyerId", AttributeValue::S(curr_bid_ref.buyer_id.clone()))
        .key("id", AttributeValue::S(curr_bid_ref.id.to_string()))
        .send()
        .await?;
    let db_bid = get_bid_resp.item.ok_or(HandlerError::not_found())?;
    let bid: Bid = from_item(db_bid)?;

    let seller_income = ((bid.amount as f64) * 0.95).floor() as u64;
    let now_ts = chrono::Local::now().timestamp_millis() as u64;
    let purchase = Purchase {
        buyer_id: bid.buyer_id.clone(),
        id: Ulid::new(),
        create_at: now_ts,
        item: ItemRef {
            seller_id: claim.id.clone(),
            id: item_id,
        },
        price: bid.amount,
        sold_time: bid.create_at,
    };

    let seller_update = TransactWriteItem::builder()
        .update(
            Update::builder()
                .table_name(SELLER_TABLE)
                .key("id", AttributeValue::S(claim.id.clone()))
                .update_expression("SET fund = fund + :amount")
                .expression_attribute_values(":amount", to_attribute_value(seller_income)?)
                .build()?,
        )
        .build();

    let buyer_update = TransactWriteItem::builder()
        .update(
            Update::builder()
                .table_name(BUYER_TABLE)
                .key("id", AttributeValue::S(bid.buyer_id.clone()))
                .update_expression("SET fundOnHold = fundOnHold - :amount")
                .condition_expression("fundOnHold >= :amount")
                .expression_attribute_values(":amount", to_attribute_value(bid.amount)?)
                .build()?,
        )
        .build();

    let purchase_put = TransactWriteItem::builder()
        .put(
            Put::builder()
                .table_name(PURCHASE_TABLE)
                .set_item(Some(to_item(purchase)?))
                .build()?,
        )
        .build();

    let item_update = TransactWriteItem::builder()
        .update(
            Update::builder()
                .table_name(ITEM_TABLE)
                .key("sellerId", AttributeValue::S(claim.id.clone()))
                .key("id", AttributeValue::S(item_id.to_string()))
                .update_expression("SET soldBid = :bid_ref, soldTime = :time, soldPrice = :price, state = :archived")
                .expression_attribute_values(":bid_ref", to_attribute_value(curr_bid_ref.clone())?)
                .expression_attribute_values(":time", to_attribute_value(bid.create_at)?)
                .expression_attribute_values(":price", to_attribute_value(bid.amount)?)
                .expression_attribute_values(":state", to_attribute_value(ItemState::Archived)?)
                .build()?,
        )
        .build();

    let bid_update = TransactWriteItem::builder()
        .update(
            Update::builder()
                .table_name(BID_TABLE)
                .key("buyerId", AttributeValue::S(curr_bid_ref.buyer_id.clone()))
                .key("id", AttributeValue::S(curr_bid_ref.id.to_string()))
                .update_expression("SET isActive = :false")
                .expression_attribute_values(":false", AttributeValue::Bool(false))
                .build()?,
        )
        .build();

    client
        .transact_write_items()
        .transact_items(seller_update)
        .transact_items(buyer_update)
        .transact_items(purchase_put)
        .transact_items(item_update)
        .transact_items(bid_update)
        .send()
        .await?;

    Ok(())
}

/// Archive item by itemId.
#[utoipa::path(
    post,
    path = "/item/{itemId}/archive",
    tag = "Seller",
    params(
        ("itemId" = String, Path, description = "Item ID to archive", format = Ulid),
    ),
    responses(
        (status = OK, description = "Item archive success"),
        (status = FORBIDDEN, description = "Not a seller", body = HandlerError),
        (status = NOT_FOUND, description = "Item not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn seller_archive_item_by_id(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<Ulid>,
) -> Result<(), HandlerError> {
    check_user(claim.as_claim())?;

    let client = Client::new(&state.aws_config);

    client
        .update_item()
        .key("sellerId", AttributeValue::S(claim.id))
        .key("id", AttributeValue::S(item_id.to_string()))
        .update_expression("SET state = :archived")
        .condition_expression("state = :inactive OR state = :failed")
        .expression_attribute_values(":archived", ItemState::Active.into())
        .expression_attribute_values(":inactive", ItemState::InActive.into())
        .expression_attribute_values(":failed", ItemState::Failed.into())
        .send()
        .await?;

    Ok(())
}
