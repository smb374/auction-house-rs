use axum::http::StatusCode;

use crate::{
    errors::HandlerError,
    models::{auth::Claim, user::UserType},
};

pub mod auth;
pub mod buyer;
pub mod item;
pub mod seller;

fn check_user(claim: Claim, user_type: UserType) -> Result<(), HandlerError> {
    if claim.user_type != user_type {
        return Err(HandlerError::HandlerError(
            StatusCode::FORBIDDEN,
            format!("Only {} can use this.", user_type),
        ));
    }
    Ok(())
}
