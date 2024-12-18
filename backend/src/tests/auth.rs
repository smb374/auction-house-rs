use std::sync::Arc;

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lambda_http::{tower::ServiceExt, Error};
use ulid::Ulid;

use crate::{
    constants::{BUYER_TABLE, SELLER_TABLE},
    create_service,
    models::{
        auth::{LoginPayload, RegisterPayload},
        user::{UserInfo, UserType},
    },
    state::AppState,
    tests::parse_resp,
    utils::create_userid,
};

async fn clean_account(
    state: Arc<AppState>,
    email: String,
    user_type: UserType,
) -> Result<(), Error> {
    let table = match user_type {
        UserType::Buyer => BUYER_TABLE,
        UserType::Seller => SELLER_TABLE,
        UserType::Admin => unreachable!(),
    };

    let client = Client::new(&state.aws_config);
    let id = create_userid(&email, user_type);

    client
        .delete_item()
        .table_name(table)
        .key("id", AttributeValue::S(id))
        .send()
        .await?;
    Ok(())
}

#[tokio::test]
async fn test_oneshot() -> Result<(), Error> {
    let state = Arc::new(AppState::new().await?);
    let service = create_service(state).await?;
    let request = Request::builder().uri("/v1/").body(Body::empty())?;

    let response = service.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_auth_login() -> Result<(), Error> {
    let state = Arc::new(AppState::new().await?);
    let random_email = format!("test_seller_{}@test.com", Ulid::new());
    let password_str = Ulid::new();
    {
        let service = create_service(state.clone()).await?;

        let register_payload = RegisterPayload {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: random_email.clone(),
            user_type: UserType::Seller,
            password: password_str.to_string(),
        };

        let payload: String = serde_json::to_string(&register_payload)?;

        let req = Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .uri("/v1/register")
            .body(payload)?;

        let resp = service.oneshot(req).await?;

        assert_eq!(resp.status(), StatusCode::OK);

        let user_info: UserInfo = parse_resp(resp).await?;
        assert_eq!(user_info.email.as_str(), random_email.as_str());
    }

    // Login
    {
        let service = create_service(state.clone()).await?;

        let answer = LoginPayload {
            email: random_email.clone(),
            user_type: UserType::Seller,
            password: password_str.to_string(),
        };
        let payload: String = serde_json::to_string(&answer)?;

        let req = Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .uri("/v1/login")
            .body(payload)?;

        let resp = service.oneshot(req).await?;

        assert_eq!(resp.status(), StatusCode::OK);

        let user_info: UserInfo = parse_resp(resp).await?;
        assert_eq!(user_info.email.as_str(), random_email.as_str());
    }

    clean_account(state, random_email, UserType::Seller).await?;

    Ok(())
}
