use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::user::UserType;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPayload {
    /// User first name
    pub first_name: String,
    /// User last name
    pub last_name: String,
    /// User Email
    pub email: String,
    /// User type of the user.
    pub user_type: UserType,
    /// Password in bcrypt
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginPayload {
    /// User Email
    pub email: String,
    /// User type of the user.
    pub user_type: UserType,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct LoginChallenge {
    /// Salt for the user to hash the password
    pub salt: String,
    /// log_n param of scrypt.
    pub log_n: u8,
    /// r param of scrypt.
    pub r: u32,
    /// p param of scrypt.
    pub p: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginChallengeAnswer {
    /// User Email
    pub email: String,
    /// User type of the user.
    pub user_type: UserType,
    /// User hashed password.
    pub password_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Claim<'a> {
    /// ID
    pub id: &'a str,
    /// User first name
    pub first_name: &'a str,
    /// User last name
    pub last_name: &'a str,
    /// User Email
    pub email: &'a str,
    /// User type of the user.
    pub user_type: UserType,
    /// Expire Time
    pub exp: u64,
    /// Issue Time
    pub iat: u64,
}
