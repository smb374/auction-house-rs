use std::sync::Arc;

use aws_sdk_dynamodb::{
    operation::transact_write_items::builders::TransactWriteItemsFluentBuilder,
    types::{AttributeValue, Delete, TransactWriteItem},
    Client,
};
use axum::http::{Request, StatusCode};
use chrono::TimeDelta;
use lambda_http::{tower::ServiceExt, Error};
use serde_dynamo::from_items;
use ulid::Ulid;

use crate::{
    constants::ITEM_TABLE,
    create_service,
    models::{
        auth::LoginPayload,
        item::{AddItemRequest, Item, ItemRef},
        user::{UserInfo, UserType},
    },
    state::AppState,
    tests::{build_request, parse_resp},
};

const TEST_SELLER_EMAIL: &str = "foo@test.org";
const TEST_SELLER_PASSWORD: &str = "01JFDQ42PN3MDE6QMPZ98TCTJE";

pub async fn clean_item(state: Arc<AppState>, id: String, item_id: Ulid) -> Result<(), Error> {
    let client = Client::new(&state.aws_config);

    client
        .delete_item()
        .table_name(ITEM_TABLE)
        .key("sellerId", AttributeValue::S(id))
        .key("id", AttributeValue::S(item_id.to_string()))
        .send()
        .await?;

    Ok(())
}

pub async fn clean_items(state: Arc<AppState>, id: String) -> Result<(), Error> {
    let client = Client::new(&state.aws_config);

    let query_resp = client
        .query()
        .table_name(ITEM_TABLE)
        .key_condition_expression("sellerId = :sid")
        .expression_attribute_values(":sid", AttributeValue::S(id.clone()))
        .send()
        .await?;

    let items: Vec<Item> = from_items(query_resp.items().to_vec())?;

    let transactions = items.into_iter().try_fold(
        client.transact_write_items(),
        |acc, item| -> Result<TransactWriteItemsFluentBuilder, Error> {
            let nacc = acc.transact_items(
                TransactWriteItem::builder()
                    .delete(
                        Delete::builder()
                            .table_name(ITEM_TABLE)
                            .key("sellerId", AttributeValue::S(id.clone()))
                            .key("id", AttributeValue::S(item.id.to_string()))
                            .build()?,
                    )
                    .build(),
            );
            Ok(nacc)
        },
    )?;

    transactions.send().await?;

    Ok(())
}

pub async fn add_test_item<S: Into<String>>(
    state: Arc<AppState>,
    user_info: &UserInfo,
    name: S,
) -> Result<ItemRef, Error> {
    let service = create_service(state.clone()).await?;

    let add_item_req = AddItemRequest {
        name: name.into(),
        description: "A test item".to_string(),
        init_price: 100,
        auction_length: TimeDelta::minutes(1).num_milliseconds() as u64,
        images: Vec::new(),
    };

    let req = build_request(
        "PUT",
        "/v1/seller/item",
        &user_info.token,
        Some(add_item_req),
    )?;
    let resp = service.oneshot(req).await?;

    assert_eq!(resp.status(), StatusCode::OK);

    let item_ref: ItemRef = parse_resp(resp).await?;

    assert_eq!(&item_ref.seller_id, &user_info.id);

    Ok(item_ref)
}

pub async fn test_seller_login(state: Arc<AppState>) -> Result<UserInfo, Error> {
    let service = create_service(state.clone()).await?;
    let login_payload = LoginPayload {
        email: TEST_SELLER_EMAIL.to_string(),
        user_type: UserType::Seller,
        password: TEST_SELLER_PASSWORD.to_string(),
    };
    let payload: String = serde_json::to_string(&login_payload)?;

    let req = Request::builder()
        .method("POST")
        .header("Content-Type", "application/json")
        .uri("/v1/login")
        .body(payload)?;

    let resp = service.oneshot(req).await?;

    assert_eq!(resp.status(), StatusCode::OK);

    let user_info: UserInfo = parse_resp(resp).await?;

    Ok(user_info)
}

#[tokio::test]
async fn test_seller_add_item() -> Result<(), Error> {
    let state = Arc::new(AppState::new().await?);

    let user_info = test_seller_login(state.clone()).await?;

    let item_ref = add_test_item(state.clone(), &user_info, "TestItem").await?;

    clean_item(state, item_ref.seller_id, item_ref.id).await?;
    Ok(())
}

#[tokio::test]
async fn test_seller_get_items() -> Result<(), Error> {
    let state = Arc::new(AppState::new().await?);

    let user_info = test_seller_login(state.clone()).await?;

    for _ in 0..16 {
        add_test_item(state.clone(), &user_info, Ulid::new()).await?;
    }
    let service = create_service(state.clone()).await?;

    let req = build_request::<()>("GET", "/v1/seller/item", &user_info.token, None)?;
    let resp = service.oneshot(req).await?;

    let resp_items: Vec<Item> = parse_resp(resp).await?;

    assert!(resp_items.len() >= 16);

    clean_items(state, user_info.id).await?;

    Ok(())
}
