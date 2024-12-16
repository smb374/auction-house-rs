use std::sync::Arc;

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};

use crate::{
    constants::{BUYER_TABLE, SELLER_TABLE},
    models::{
        auth::{Claim, RegisterPayload},
        user::{Buyer, Seller, UserInfo, UserType},
        ErrorResponse, GeneralResult,
    },
    state::AppState,
    utils::create_userid,
};

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterPayload>,
) -> GeneralResult<Json<UserInfo>> {
    let client = Client::new(&state.aws_config);
    let table = match payload.user_type {
        UserType::Buyer => BUYER_TABLE,
        UserType::Seller => SELLER_TABLE,
    };
    // Get deterministic user id.
    let id = create_userid(&payload.email, payload.user_type);
    // 1. Check if user exists
    let get_user_resp = client
        .get_item()
        .table_name(table)
        .key("id", AttributeValue::S(id.clone()))
        .send()
        .await
        .map_err(|e| ErrorResponse {
            status: StatusCode::INSUFFICIENT_STORAGE.as_u16(),
            inner_status: e.raw_response().map(|r| r.status().as_u16()),
            message: format!("Error getting user: {}", e),
        })?;

    if get_user_resp.item().is_some() {
        return Err(ErrorResponse {
            status: StatusCode::BAD_REQUEST.as_u16(),
            inner_status: None,
            message: "User already exists!".to_string(),
        });
    }

    // 2. Check if password is a valid bcrypt hash.
    let phash =
        bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST).map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: None,
            message: format!("Failed to hash password: {}", e),
        })?;

    // 3. Create user.
    let now = chrono::Local::now();
    let current = now.timestamp_millis() as u64;
    let user_item = match payload.user_type {
        UserType::Buyer => {
            let user = Buyer {
                id: id.clone(),
                create_at: current,
                is_active: true,
                first_name: payload.first_name.clone(),
                last_name: payload.last_name.clone(),
                email: payload.email.clone(),
                fund: 0,
                bids: Vec::new(),
                purchases: Vec::new(),
                password: phash,
            };
            serde_dynamo::to_item(user)
        }
        UserType::Seller => {
            let user = Seller {
                id: id.clone(),
                create_at: current,
                is_active: true,
                first_name: payload.first_name.clone(),
                last_name: payload.last_name.clone(),
                email: payload.email.clone(),
                fund: 0,
                auctions: Vec::new(),
                password: phash,
            };
            serde_dynamo::to_item(user)
        }
    }
    .map_err(|e| ErrorResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        inner_status: None,
        message: format!("Failed to serialize user: {}", e),
    })?;

    // 4. Write item.
    client
        .put_item()
        .table_name(table)
        .set_item(Some(user_item))
        .send()
        .await
        .map_err(|e| ErrorResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            inner_status: e.raw_response().map(|r| r.status().as_u16()),
            message: format!("Error putting user: {}", e),
        })?;

    // 5. Sign JWT token.
    let enc_key = &state.jwt.0;
    let header = &state.jwt.2;
    let claim = Claim {
        id: id.clone(),
        first_name: payload.first_name.clone(),
        last_name: payload.last_name.clone(),
        email: payload.email.clone(),
        user_type: payload.user_type,
        iat: current,
        exp: (now + std::time::Duration::from_secs(60 * 60)).timestamp_millis() as u64, // 1 hr.
    };

    let token = jsonwebtoken::encode(header, &claim, enc_key).map_err(|e| ErrorResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        inner_status: None,
        message: format!("Failed to sign JWT token: {}", e),
    })?;

    Ok(Json(UserInfo {
        id,
        first_name: payload.first_name,
        last_name: payload.last_name,
        email: payload.email,
        user_type: payload.user_type,
        token,
    }))
}

pub async fn login_challenge(State(state): State<Arc<AppState>>) {}
