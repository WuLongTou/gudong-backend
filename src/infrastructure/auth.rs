use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub username: String,
    pub exp: usize,
}

pub fn validate_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret("secret".as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .ok()
}

pub fn generate_token(user_id: &str, username: &str) -> Result<String, AppError> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("Invalid timestamp")
        .timestamp() as usize;
    
    let claims = Claims {
        user_id: user_id.to_owned(),
        username: username.to_owned(),
        exp,
    };
    
    encode(&Header::default(), &claims, &EncodingKey::from_secret("secret".as_ref()))
        .map_err(|_| AppError::InternalServerError)
} 