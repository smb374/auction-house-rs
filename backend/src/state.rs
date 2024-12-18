use std::env;

use aws_config::{BehaviorVersion, Region, SdkConfig};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};
use lambda_http::Error;

pub struct AppState {
    pub aws_config: SdkConfig,
    pub jwt: (EncodingKey, DecodingKey, Header),
}

impl AppState {
    pub async fn new() -> Result<Self, Error> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .load()
            .await;
        let secret = env::var("JWT_SECRET").map_err(|e| e.to_string())?;

        Ok(Self {
            aws_config: config,
            jwt: (
                EncodingKey::from_base64_secret(&secret)?,
                DecodingKey::from_base64_secret(&secret)?,
                Header::new(Algorithm::HS256),
            ),
        })
    }

    pub async fn test() -> Result<Self, Error> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url("http://localhost:8000")
            .region(Region::new("test"))
            .load()
            .await;
        let secret = env::var("JWT_SECRET").map_err(|e| e.to_string())?;

        Ok(Self {
            aws_config: config,
            jwt: (
                EncodingKey::from_base64_secret(&secret)?,
                DecodingKey::from_base64_secret(&secret)?,
                Header::new(Algorithm::HS256),
            ),
        })
    }
}
