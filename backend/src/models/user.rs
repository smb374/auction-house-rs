use std::fmt;

use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::HandlerError;

use super::auth::Claim;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserType {
    Seller,
    Buyer,
    Admin,
}

impl fmt::Display for UserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UserType::Buyer => write!(f, "buyer"),
            UserType::Seller => write!(f, "seller"),
            UserType::Admin => write!(f, "admin"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserWrapper {
    Seller(Seller),
    Buyer(Buyer),
}

impl UserWrapper {
    pub fn create_claim(&self, exp: TimeDelta) -> Claim<'_> {
        let now = chrono::Local::now();
        match self {
            UserWrapper::Buyer(user) => Claim {
                id: &user.id,
                first_name: &user.first_name,
                last_name: &user.last_name,
                email: &user.email,
                user_type: UserType::Buyer,
                iat: now.timestamp_millis() as u64,
                exp: (now + exp).timestamp_millis() as u64,
                aud: "auction-house-rs",
            },
            UserWrapper::Seller(user) => Claim {
                id: &user.id,
                first_name: &user.first_name,
                last_name: &user.last_name,
                email: &user.email,
                user_type: UserType::Seller,
                iat: now.timestamp_millis() as u64,
                exp: (now + exp).timestamp_millis() as u64,
                aud: "auction-house-rs",
            },
        }
    }

    pub fn password(&self) -> &str {
        match self {
            UserWrapper::Buyer(user) => &user.password,
            UserWrapper::Seller(user) => &user.password,
        }
    }

    pub fn to_item<I: From<serde_dynamo::Item>>(self) -> Result<I, HandlerError> {
        let res = match self {
            UserWrapper::Buyer(user) => serde_dynamo::to_item(user)?,
            UserWrapper::Seller(user) => serde_dynamo::to_item(user)?,
        };
        Ok(res)
    }

    pub fn to_user_info(self, token: String) -> UserInfo {
        match self {
            UserWrapper::Buyer(user) => UserInfo {
                id: user.id,
                first_name: user.first_name,
                last_name: user.last_name,
                email: user.email,
                user_type: UserType::Buyer,
                token,
            },
            UserWrapper::Seller(user) => UserInfo {
                id: user.id,
                first_name: user.first_name,
                last_name: user.last_name,
                email: user.email,
                user_type: UserType::Seller,
                token,
            },
        }
    }
}

impl From<Buyer> for UserWrapper {
    fn from(value: Buyer) -> Self {
        Self::Buyer(value)
    }
}

impl From<Seller> for UserWrapper {
    fn from(value: Seller) -> Self {
        Self::Seller(value)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    /// ID
    pub id: String,
    /// User first name
    pub first_name: String,
    /// User last name
    pub last_name: String,
    /// User Email
    pub email: String,
    /// User type of the returned user.
    pub user_type: UserType,
    /// Signed JWT token.
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Seller {
    /// ID
    pub id: String,
    /// Create time, in unix timestamp
    pub create_at: u64,
    /// User is active
    pub is_active: bool,
    /// User first name
    pub first_name: String,
    /// User last name
    pub last_name: String,
    /// User Email
    pub email: String,
    /// User fund
    pub fund: u64,
    /// Password in scrypt.
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Buyer {
    /// ID
    pub id: String,
    /// Create time, in unix timestamp
    pub create_at: u64,
    /// User is active
    pub is_active: bool,
    /// User first name
    pub first_name: String,
    /// User last name
    pub last_name: String,
    /// User Email
    pub email: String,
    /// User fund
    pub fund: u64,
    /// User fund on hold
    pub fund_on_hold: u64,
    /// Password in scrypt.
    pub password: String,
}
