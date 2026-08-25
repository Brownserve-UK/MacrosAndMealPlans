use std::sync::Arc;

use async_trait::async_trait;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mmp_core::services::HouseholdService;

use super::{AuthError, AuthProvider, Principal};

// 2026-08-22: Development only, basic auth will be replaced by OpenID Connect before release
pub struct DevBasicAuthProvider {
    household: Arc<HouseholdService>,
    password: String,
}

impl DevBasicAuthProvider {
    pub fn new(household: Arc<HouseholdService>, password: impl Into<String>) -> Self {
        Self {
            household,
            password: password.into(),
        }
    }
}

#[async_trait]
impl AuthProvider for DevBasicAuthProvider {
    async fn authenticate(&self, headers: &HeaderMap) -> Result<Principal, AuthError> {
        let (username, password) = read_basic(headers)?;

        if !constant_time_eq(password.as_bytes(), self.password.as_bytes()) {
            return Err(AuthError::InvalidCredentials);
        }

        let user = self
            .household
            .find_user_by_username(&username)
            .await
            .map_err(|_| AuthError::Unavailable)?
            .ok_or(AuthError::InvalidCredentials)?;

        if user.is_archived() {
            return Err(AuthError::InvalidCredentials);
        }

        let member = self
            .household
            .find_member_by_linked_user(user.id)
            .await
            .map_err(|_| AuthError::Unavailable)?;

        Ok(Principal::from_user(&user, member.map(|m| m.id)))
    }
}

fn read_basic(headers: &HeaderMap) -> Result<(String, String), AuthError> {
    let header = headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingCredentials)?
        .to_str()
        .map_err(|_| AuthError::MalformedCredentials("the header was not valid ASCII"))?;

    let encoded = header
        .strip_prefix("Basic ")
        .ok_or(AuthError::MalformedCredentials("expected a Basic scheme"))?;

    let decoded = STANDARD
        .decode(encoded.trim())
        .map_err(|_| AuthError::MalformedCredentials("the payload was not valid base64"))?;
    let decoded = String::from_utf8(decoded)
        .map_err(|_| AuthError::MalformedCredentials("the payload was not valid UTF-8"))?;

    let (username, password) = decoded
        .split_once(':')
        .ok_or(AuthError::MalformedCredentials("expected `user:password`"))?;

    Ok((username.to_owned(), password.to_owned()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
#[path = "dev_tests.rs"]
mod tests;
