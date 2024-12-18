use std::{collections::HashMap, sync::Arc};

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::{Duration, TimeDelta};
use scrypt::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Scrypt,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::{BUYER_TABLE, SELLER_TABLE},
    errors::HandlerError,
    models::{
        auth::{LoginPayload, RegisterPayload},
        user::{Buyer, Seller, UserInfo, UserType, UserWrapper},
    },
    state::AppState,
    utils::create_userid,
};

const TOKEN_EXPIRATION_DURATION: TimeDelta = Duration::hours(5);

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(login))
}

async fn get_user(
    client: &Client,
    id: &str,
    table: &str,
) -> Result<Option<HashMap<String, AttributeValue>>, HandlerError> {
    let resp = client
        .get_item()
        .table_name(table)
        .key("id", AttributeValue::S(id.to_string()))
        .send()
        .await?;

    Ok(resp.item)
}

async fn get_user_full(
    client: &Client,
    id: &str,
    table: &str,
    user_type: UserType,
) -> Result<UserWrapper, HandlerError> {
    let get_user_resp = get_user(&client, &id, table).await?;
    let user_item = get_user_resp.ok_or(HandlerError::not_found())?;
    match user_type {
        UserType::Buyer => {
            let buyer: Buyer = serde_dynamo::from_item(user_item)?;
            Ok(UserWrapper::from(buyer))
        }
        UserType::Seller => {
            let seller: Seller = serde_dynamo::from_item(user_item)?;
            Ok(UserWrapper::from(seller))
        }
        UserType::Admin => unreachable!(),
    }
}

/// Register user account.
#[utoipa::path(
    post,
    path = "/v1/register",
    tag = "Auth",
    request_body(description = "Register Info", content = RegisterPayload),
    responses(
        (status = OK, description = "Register Success", body = UserInfo),
        (status = BAD_REQUEST, description = "User already exists", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<UserInfo>, HandlerError> {
    let client = Client::new(&state.aws_config);
    let id = create_userid(&payload.email, payload.user_type);
    let table = match payload.user_type {
        UserType::Buyer => BUYER_TABLE,
        UserType::Seller => SELLER_TABLE,
        UserType::Admin => unreachable!(),
    };

    // 1. Check user existance.
    let get_user_resp = get_user(&client, &id, table).await?;
    if get_user_resp.is_some() {
        return Err(HandlerError::HandlerError(
            StatusCode::BAD_REQUEST,
            "User already exists".to_string(),
        ));
    }

    // 2. Create password hash.
    let salt = SaltString::generate(&mut OsRng);
    let phash = Scrypt
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    // 3. Create user.
    let now = chrono::Local::now();
    let current = now.timestamp_millis() as u64;
    let user = match payload.user_type {
        UserType::Buyer => UserWrapper::from(Buyer {
            id: id.clone(),
            create_at: current,
            is_active: true,
            first_name: payload.first_name.clone(),
            last_name: payload.last_name.clone(),
            email: payload.email.clone(),
            fund: 0,
            fund_on_hold: 0,
            password: phash,
        }),
        UserType::Seller => UserWrapper::from(Seller {
            id: id.clone(),
            create_at: current,
            is_active: true,
            first_name: payload.first_name.clone(),
            last_name: payload.last_name.clone(),
            email: payload.email.clone(),
            fund: 0,
            password: phash,
        }),
        UserType::Admin => unreachable!(),
    };
    let user_item = user.clone().to_item()?;

    // 4. Write item.
    client
        .put_item()
        .table_name(table)
        .set_item(Some(user_item))
        .send()
        .await?;

    // 5. Sign JWT token.
    let enc_key = &state.jwt.0;
    let header = &state.jwt.2;
    let claim = user.create_claim(TOKEN_EXPIRATION_DURATION);

    let token = jsonwebtoken::encode(header, &claim, enc_key)?;

    Ok(Json(user.to_user_info(token)))
}

/// User Login
#[utoipa::path(
    post,
    path = "/v1/login",
    tag = "Auth",
    request_body(description = "Register Info", content = LoginPayload),
    responses(
        (status = OK, description = "Login Success", body = UserInfo),
        (status = BAD_REQUEST, description = "Wrong password or malformed password hash", body = HandlerError),
        (status = NOT_FOUND, description = "User not found", body = HandlerError),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = HandlerError),
    ),
)]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<UserInfo>, HandlerError> {
    let client = Client::new(&state.aws_config);
    let id = create_userid(&payload.email, payload.user_type);
    let table = match payload.user_type {
        UserType::Buyer => BUYER_TABLE,
        UserType::Seller => SELLER_TABLE,
        UserType::Admin => unreachable!(),
    };

    // 1. Check if user exists
    let user = get_user_full(&client, &id, table, payload.user_type).await?;

    // 2. verify hash
    let phash = PasswordHash::new(user.password())?;

    Scrypt.verify_password(payload.password.as_bytes(), &phash)?;

    // 3. Sign JWT token
    let enc_key = &state.jwt.0;
    let header = &state.jwt.2;
    let claim = user.create_claim(TOKEN_EXPIRATION_DURATION);

    let token = jsonwebtoken::encode(header, &claim, enc_key)?;

    Ok(Json(user.to_user_info(token)))
}
