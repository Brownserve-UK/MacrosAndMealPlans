use std::fmt;

use crate::domain::Revision;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{resource} `{id}` was not found")]
    NotFound { resource: &'static str, id: String },

    #[error("{resource} `{id}` has changed since you last loaded it")]
    RevisionMismatch {
        resource: &'static str,
        id: String,
        expected: Revision,
        actual: Revision,
    },

    #[error("a {resource} with that {field} already exists")]
    Duplicate {
        resource: &'static str,
        field: &'static str,
        value: String,
    },

    #[error("{detail}")]
    Conflict { detail: String },

    #[error("the supplied values were not valid")]
    Validation(#[from] ValidationErrors),

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

impl CoreError {
    pub fn not_found(resource: &'static str, id: impl fmt::Display) -> Self {
        Self::NotFound {
            resource,
            id: id.to_string(),
        }
    }

    pub fn duplicate(
        resource: &'static str,
        field: &'static str,
        value: impl fmt::Display,
    ) -> Self {
        Self::Duplicate {
            resource,
            field,
            value: value.to_string(),
        }
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::Conflict {
            detail: detail.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct RepositoryError {
    message: String,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl RepositoryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, thiserror::Error)]
pub struct ValidationErrors(Vec<ValidationError>);

impl ValidationErrors {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.0.push(ValidationError {
            field: field.into(),
            message: message.into(),
        });
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.0.iter()
    }

    pub fn into_result(self) -> Result<()> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(CoreError::Validation(self))
        }
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined: Vec<String> = self
            .0
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect();
        write!(f, "{}", joined.join(", "))
    }
}
