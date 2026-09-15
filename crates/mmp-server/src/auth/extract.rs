use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use super::Principal;
use crate::error::ApiError;
use crate::state::AppState;

impl FromRequestParts<AppState> for Principal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        state
            .auth
            .authenticate(&parts.headers)
            .await
            .map_err(ApiError::from)
    }
}
