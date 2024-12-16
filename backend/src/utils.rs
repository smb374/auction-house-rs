use base64::{prelude::BASE64_URL_SAFE, Engine};
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake128,
};

use crate::models::user::UserType;

pub fn create_userid(email: &str, user_type: UserType) -> String {
    let mut hasher = Shake128::default();
    hasher.update(email.as_bytes());
    hasher.update(user_type.to_string().as_bytes());
    let mut reader = hasher.finalize_xof();
    let mut buf = [0u8; 12];
    reader.read(&mut buf);
    let suffix = BASE64_URL_SAFE.encode(&buf);
    format!("{}_{}", user_type.to_string(), suffix)
}
