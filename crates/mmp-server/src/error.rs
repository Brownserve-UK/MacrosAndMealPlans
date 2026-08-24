use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use mmp_core::{CoreError, ValidationErrors};
use serde::Serialize;

use crate::auth::AuthError;

const PROBLEM_BASE: &str = "https://macrosandmealplans.dev/problems";

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Problem {
    #[schema(example = "https://macrosandmealplans.dev/problems/validation-failed")]
    pub r#type: String,
    #[schema(example = "Validation failed")]
    pub title: String,
    #[schema(example = 422)]
    pub status: u16,
    pub detail: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<FieldProblem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_revision: Option<i64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FieldProblem {
    #[schema(example = "name")]
    pub field: String,
    #[schema(example = "Required")]
    pub message: String,
}

#[derive(Debug)]
pub struct ApiError(Box<Problem>);

impl ApiError {
    pub fn new(status: StatusCode, kind: &str, title: &str, detail: impl Into<String>) -> Self {
        Self(Box::new(Problem {
            r#type: format!("{PROBLEM_BASE}/{kind}"),
            title: title.to_owned(),
            status: status.as_u16(),
            detail: detail.into(),
            errors: Vec::new(),
            expected_revision: None,
            actual_revision: None,
        }))
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "bad-request",
            "Bad request",
            detail,
        )
    }

    pub fn precondition_required(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::PRECONDITION_REQUIRED,
            "precondition-required",
            "Precondition required",
            detail,
        )
    }

    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.0.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn with_fields(mut self, errors: &ValidationErrors) -> Self {
        self.0.errors = errors
            .iter()
            .map(|e| FieldProblem {
                field: e.field.clone(),
                message: e.message.clone(),
            })
            .collect();
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let mut response = (status, Json(&*self.0)).into_response();
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::NotFound { resource, ref id } => {
                tracing::debug!(%resource, %id, "not found");
                ApiError::new(
                    StatusCode::NOT_FOUND,
                    "not-found",
                    "Not found",
                    format!("That {resource} no longer exists."),
                )
            }
            CoreError::RevisionMismatch {
                resource,
                ref id,
                expected,
                actual,
            } => {
                let mut problem = ApiError::new(
                    StatusCode::CONFLICT,
                    "revision-conflict",
                    "Revision conflict",
                    format!("Someone else changed this {resource} first."),
                );
                problem.0.expected_revision = Some(expected.get());
                problem.0.actual_revision = Some(actual.get());
                let _ = id;
                problem
            }
            CoreError::Duplicate {
                resource,
                field,
                ref value,
            } => {
                tracing::debug!(%resource, %field, %value, "duplicate rejected");
                ApiError::new(
                    StatusCode::CONFLICT,
                    "duplicate",
                    "Already exists",
                    format!("Another {resource} already uses that {field}."),
                )
            }
            CoreError::Conflict { ref detail } => {
                ApiError::new(StatusCode::CONFLICT, "conflict", "Conflict", detail.clone())
            }
            CoreError::Validation(ref errors) => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation-failed",
                "Validation failed",
                "Check the highlighted fields.",
            )
            .with_fields(errors),
            CoreError::Repository(ref inner) => {
                tracing::error!(error = %inner, "repository failure");
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal server error",
                    "Something went wrong.",
                )
            }
        }
    }
}

impl From<AuthError> for ApiError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::Forbidden(_) => ApiError::new(
                StatusCode::FORBIDDEN,
                "forbidden",
                "Forbidden",
                error.to_string(),
            ),
            AuthError::MissingCredentials | AuthError::InvalidCredentials => ApiError::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized",
                error.to_string(),
            ),
            AuthError::MalformedCredentials(_) => ApiError::new(
                StatusCode::BAD_REQUEST,
                "malformed-credentials",
                "Malformed credentials",
                error.to_string(),
            ),
            AuthError::Unavailable => {
                tracing::error!("the account store could not be reached during authentication");
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal server error",
                    "Something went wrong.",
                )
            }
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
