use std::sync::Arc;

use aws_sdk_dynamodb::{
    types::{AttributeValue, Put, ReturnValue, TransactWriteItem, Update},
    Client,
};
use axum::{extract::State, http::StatusCode, Extension, Json};
use serde_dynamo::{from_attribute_value, from_item, to_attribute_value, to_item};
use ulid::Ulid;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::{BID_TABLE, BUYER_TABLE, ITEM_TABLE},
    errors::HandlerError,
    models::{
        auth::ClaimOwned,
        bid::{Bid, BidItemRequest, BidRef},
        buyer::{AddFundRequest, AddFundResponse},
        item::{ItemRef, ItemState},
        user::UserType,
    },
    state::AppState,
};

use super::check_user;

pub fn route() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(buyer_add_fund))
        .routes(routes!(buyer_place_bid))
}

/// Add fund to buyer
#[utoipa::path(
    post,
    path = "/add-fund",
    tag = "Buyer",
    request_body = AddFundRequest,
    responses(
        (status = OK, description = "Add fund success", body = AddFundResponse),
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
) -> Result<Json<AddFundResponse>, HandlerError> {
    check_user(claim.as_claim(), UserType::Buyer)?;

    let client = Client::new(&state.aws_config);

    let resp = client
        .update_item()
        .table_name(BUYER_TABLE)
        .key("id", AttributeValue::S(claim.id.clone()))
        .update_expression("SET fund = fund + :amount")
        .expression_attribute_values(":amount", to_attribute_value(payload.amount)?)
        .return_values(ReturnValue::UpdatedNew)
        .send()
        .await?;

    let err = Err(HandlerError::HandlerError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "DynamoDB didn't return updated attributes.".to_string(),
    ));
    match resp.attributes() {
        Some(attrs) => match attrs.get("fund") {
            Some(attr) => Ok(Json(AddFundResponse {
                id: claim.id.clone(),
                fund: from_attribute_value(attr.clone())?,
            })),
            None => err,
        },
        None => err,
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaceBidProjection {
    current_bid: Option<BidRef>,
}

/// Place bid to an item
#[utoipa::path(
    post,
    path = "/bid",
    tag = "Buyer",
    request_body = BidItemRequest,
    responses(
        (status = OK, description = "Place bid success", body = BidRef),
        (status = FORBIDDEN, description = "Not a buyer", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
    security(
        ("http-jwt" = []),
    ),
)]
async fn buyer_place_bid(
    Extension(claim): Extension<ClaimOwned>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BidItemRequest>,
) -> Result<Json<BidRef>, HandlerError> {
    check_user(claim.as_claim(), UserType::Buyer)?;

    let client = Client::new(&state.aws_config);

    let bid = Bid {
        buyer_id: claim.id.clone(),
        id: Ulid::new(),
        create_at: chrono::Local::now().timestamp_millis() as u64,
        item: ItemRef {
            seller_id: payload.seller_id.clone(),
            id: payload.id,
        },
        amount: payload.amount,
        is_active: true,
    };

    let bid_ref = BidRef::from(&bid);

    let get_item_project = client
        .get_item()
        .table_name(BID_TABLE)
        .key("sellerId", AttributeValue::S(payload.seller_id.clone()))
        .key("id", AttributeValue::S(payload.id.to_string()))
        .projection_expression("currentBid")
        .send()
        .await?;

    let project: PlaceBidProjection =
        from_item(get_item_project.item.ok_or(HandlerError::not_found())?)?;

    let put_bid = TransactWriteItem::builder()
        .put(
            Put::builder()
                .table_name(BID_TABLE)
                .set_item(Some(to_item(bid)?))
                .build()?,
        )
        .build();

    let update_buyer = TransactWriteItem::builder()
        .update(
            Update::builder()
                .table_name(BUYER_TABLE)
                .key("id", AttributeValue::S(claim.id.clone()))
                .update_expression("SET fund = fund - :amount, fundOnHold = fundOnHold + :amount")
                .condition_expression("fund >= :amount")
                .expression_attribute_values(":amount", to_attribute_value(payload.amount)?)
                .build()?,
        )
        .build();

    let update_item = TransactWriteItem::builder()
        .update(
            Update::builder()
                .table_name(ITEM_TABLE)
                .key("sellerId", AttributeValue::S(payload.seller_id))
                .key("id", AttributeValue::S(payload.id.to_string()))
                .update_expression(
                    "SET currentBid = :bid, pastBids = list_append(pastBids, :bid_list)",
                )
                .condition_expression("state = :active")
                .expression_attribute_values(":bid", to_attribute_value(bid_ref.clone())?)
                .expression_attribute_values(":bid_list", to_attribute_value([bid_ref.clone()])?)
                .expression_attribute_values(":active", ItemState::Active.into())
                .build()?,
        )
        .build();

    let transaction = client
        .transact_write_items()
        .transact_items(put_bid)
        .transact_items(update_buyer)
        .transact_items(update_item);

    match project.current_bid {
        Some(b) => {
            let update_bid = TransactWriteItem::builder()
                .update(
                    Update::builder()
                        .table_name(BID_TABLE)
                        .key("buyer_id", AttributeValue::S(b.buyer_id))
                        .key("id", AttributeValue::S(b.id.to_string()))
                        .update_expression("SET isActive = :false")
                        .expression_attribute_values(":false", AttributeValue::Bool(false))
                        .build()?,
                )
                .build();
            transaction.transact_items(update_bid)
        }
        None => transaction,
    }
    .send()
    .await?;

    Ok(Json(bid_ref))
}
