use axum::extract::FromRequestParts;
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use mmp_core::domain::Revision;
use serde::Serialize;

use crate::error::ApiError;

#[derive(Debug, Clone, Copy)]
pub struct IfMatch(pub Revision);

impl<S: Send + Sync> FromRequestParts<S> for IfMatch {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let raw = parts.headers.get(IF_MATCH).ok_or_else(|| {
            ApiError::precondition_required(
                "This request requires an `If-Match` header carrying the revision you loaded. \
                 Read the resource first and send back its `ETag`.",
            )
        })?;

        let raw = raw
            .to_str()
            .map_err(|_| ApiError::bad_request("`If-Match` was not valid ASCII."))?
            .trim();

        if raw == "*" {
            return Err(ApiError::bad_request(
                "`If-Match: *` is not accepted here. Send the concrete revision you loaded so a \
                 concurrent change cannot be overwritten unnoticed.",
            ));
        }

        let value = raw
            .strip_prefix("W/")
            .unwrap_or(raw)
            .trim()
            .trim_matches('"');

        value
            .parse::<i64>()
            .map(|v| IfMatch(Revision::new(v)))
            .map_err(|_| {
                ApiError::bad_request(format!(
                    "`If-Match` should carry a revision number, but was `{raw}`."
                ))
            })
    }
}

pub struct Tagged<T>(pub Revision, pub T);

impl<T: Serialize> IntoResponse for Tagged<T> {
    fn into_response(self) -> Response {
        let Tagged(revision, body) = self;
        let mut response = axum::Json(body).into_response();
        if let Ok(value) = HeaderValue::from_str(&format!("\"{}\"", revision.get())) {
            response.headers_mut().insert(ETAG, value);
        }
        response
    }
}

pub struct Created<T>(pub Revision, pub T);

impl<T: Serialize> IntoResponse for Created<T> {
    fn into_response(self) -> Response {
        let Created(revision, body) = self;
        let mut response = Tagged(revision, body).into_response();
        *response.status_mut() = StatusCode::CREATED;
        response
    }
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
