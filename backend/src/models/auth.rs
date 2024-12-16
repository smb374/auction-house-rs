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

pub struct LoginChallenge {}

pub struct LoginChallengeAnswer {}

pub struct LoginResponse {}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    /// ID
    pub id: String,
    /// User first name
    pub first_name: String,
    /// User last name
    pub last_name: String,
    /// User Email
    pub email: String,
    /// User type of the user.
    pub user_type: UserType,
    /// Expire Time
    pub exp: u64,
    /// Issue Time
    pub iat: u64,
}
