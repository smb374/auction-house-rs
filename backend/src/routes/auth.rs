use std::{collections::HashMap, sync::Arc};

use aws_sdk_dynamodb::{types::AttributeValue, Client};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::{Duration, TimeDelta};
use scrypt::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Scrypt,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    constants::{BUYER_TABLE, SELLER_TABLE},
    models::{
        auth::{LoginChallenge, LoginChallengeAnswer, LoginPayload, RegisterPayload},
        user::{Buyer, Seller, UserInfo, UserType, UserWrapper},
        ErrorResponse, GeneralResult,
    },
    state::AppState,
    utils::create_userid,
};

const TOKEN_EXPIRATION_DURATION: TimeDelta = Duration::hours(5);

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(login_challenge))
        .routes(routes!(login))
}

async fn get_user(
    client: &Client,
    id: &str,
    table: &str,
) -> GeneralResult<Option<HashMap<String, AttributeValue>>> {
    let resp = client
        .get_item()
        .table_name(table)
        .key("id", AttributeValue::S(id.to_string()))
        .send()
        .await
        .map_err(|e| {
            ErrorResponse::with_inner_status(
                e.raw_response().map(|r| r.status().as_u16()),
                e.to_string(),
            )
        })?;

    Ok(resp.item)
}

async fn get_user_full(
    client: &Client,
    id: &str,
    table: &str,
    user_type: UserType,
) -> GeneralResult<UserWrapper> {
    let get_user_resp = get_user(&client, &id, table).await?;
    let user_item = get_user_resp.ok_or(ErrorResponse::not_found())?;
    match user_type {
        UserType::Buyer => {
            let buyer: Buyer = serde_dynamo::from_item(user_item)
                .map_err(|e| ErrorResponse::generic("Failed to deserialize user", e))?;
            Ok(UserWrapper::from(buyer))
        }
        UserType::Seller => {
            let seller: Seller = serde_dynamo::from_item(user_item)
                .map_err(|e| ErrorResponse::generic("Failed to deserialize user", e))?;
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
        (status = BAD_REQUEST, description = "User already exists", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterPayload>,
) -> GeneralResult<Json<UserInfo>> {
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
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "User already exists.",
        ));
    }

    // 2. Create password hash.
    let salt = SaltString::generate(&mut OsRng);
    let phash = Scrypt
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| ErrorResponse::generic("Failed to hash password", e))?
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
            bids: Vec::new(),
            purchases: Vec::new(),
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
            auctions: Vec::new(),
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
        .await
        .map_err(|e| {
            ErrorResponse::with_inner_status(
                e.raw_response().map(|r| r.status().as_u16()),
                e.to_string(),
            )
        })?;

    // 5. Sign JWT token.
    let enc_key = &state.jwt.0;
    let header = &state.jwt.2;
    let claim = user.create_claim(TOKEN_EXPIRATION_DURATION);

    let token = jsonwebtoken::encode(header, &claim, enc_key)
        .map_err(|e| ErrorResponse::generic("Failed to sign JWT token", e))?;

    Ok(Json(user.to_user_info(token)))
}

/// Initiate login challenge
#[utoipa::path(
    post,
    path = "/v1/login/challenge",
    tag = "Auth",
    request_body(description = "Register Info", content = LoginPayload),
    responses(
        (status = OK, description = "Challenge Sent", body = LoginChallenge),
        (status = NOT_FOUND, description = "User not found", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn login_challenge(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> GeneralResult<Json<LoginChallenge>> {
    let client = Client::new(&state.aws_config);
    let id = create_userid(&payload.email, payload.user_type);
    let table = match payload.user_type {
        UserType::Buyer => BUYER_TABLE,
        UserType::Seller => SELLER_TABLE,
        UserType::Admin => unreachable!(),
    };

    // 1. Check if user exists
    let user = get_user_full(&client, &id, table, payload.user_type).await?;

    // 2. Get password hash
    let phash = PasswordHash::new(user.password())
        .map_err(|e| ErrorResponse::generic("Failed to parse user's password hash", e))?;

    // 3. Return salt as a challenge.
    let salt = phash
        .salt
        .expect("Scrypt password hash should have a salt.");
    let params = scrypt::Params::try_from(&phash)
        .map_err(|e| ErrorResponse::generic("Failed to parse parameter", e))?;

    Ok(Json(LoginChallenge {
        salt: salt.to_string(),
        log_n: params.log_n(),
        r: params.r(),
        p: params.p(),
    }))
}

/// Login with completed challenge
#[utoipa::path(
    post,
    path = "/v1/login",
    tag = "Auth",
    request_body(description = "Register Info", content = LoginPayload),
    responses(
        (status = OK, description = "Challenge Sent", body = LoginChallenge),
        (status = BAD_REQUEST, description = "Wrong password or malformed password hash", body = ErrorResponse),
        (status = NOT_FOUND, description = "User not found", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Handler errors", body = ErrorResponse),
    ),
)]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginChallengeAnswer>,
) -> GeneralResult<Json<UserInfo>> {
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
    let phash = PasswordHash::new(user.password())
        .map_err(|e| ErrorResponse::generic("Failed to parse user's password hash", e))?;

    let supplied = PasswordHash::new(&payload.password_hash)
        .map_err(|e| ErrorResponse::generic("Failed to parse supplied password hash", e))?;

    if phash.hash != supplied.hash || (phash.hash.is_none() && supplied.hash.is_none()) {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Wrong password",
        ));
    }

    // 4. Sign JWT token
    let enc_key = &state.jwt.0;
    let header = &state.jwt.2;
    let claim = user.create_claim(TOKEN_EXPIRATION_DURATION);

    let token = jsonwebtoken::encode(header, &claim, enc_key)
        .map_err(|e| ErrorResponse::generic("Failed to sign JWT token", e))?;

    Ok(Json(user.to_user_info(token)))
}
