use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserType {
    Seller,
    Buyer,
}

impl fmt::Display for UserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UserType::Buyer => write!(f, "buyer"),
            UserType::Seller => write!(f, "seller"),
        }
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
    pub fund: usize,
    /// List of created auctions (range keys).
    pub auctions: Vec<u128>,
    /// Password in bcrypt.
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
    pub fund: usize,
    /// List of created bids (range keys).
    pub bids: Vec<u128>,
    /// List of purchases (range keys).
    pub purchases: Vec<u128>,
    /// Password in bcrypt.
    pub password: String,
}
