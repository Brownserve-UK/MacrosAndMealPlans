mod dev;
mod extract;

pub use dev::DevBasicAuthProvider;

use std::collections::HashSet;

use async_trait::async_trait;
use axum::http::HeaderMap;
use mmp_core::domain::{HouseholdMemberId, User, UserId};

pub use mmp_core::domain::{Permission, Role};

#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: UserId,
    pub username: String,
    pub member_id: Option<HouseholdMemberId>,
    pub roles: Vec<Role>,
    pub permissions: HashSet<Permission>,
}

impl Principal {
    pub fn new(
        user_id: UserId,
        username: impl Into<String>,
        member_id: Option<HouseholdMemberId>,
        roles: Vec<Role>,
    ) -> Self {
        let permissions = roles
            .iter()
            .flat_map(|role| role.permissions().iter().copied())
            .collect();
        Self {
            user_id,
            username: username.into(),
            member_id,
            roles,
            permissions,
        }
    }

    pub fn from_user(user: &User, member_id: Option<HouseholdMemberId>) -> Self {
        Self::new(
            user.id,
            user.username.clone(),
            member_id,
            user.roles.clone(),
        )
    }

    pub fn has(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    pub fn require(&self, permission: Permission) -> Result<(), AuthError> {
        if self.has(permission) {
            Ok(())
        } else {
            Err(AuthError::Forbidden(permission))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("authentication is required")]
    MissingCredentials,
    #[error("the supplied credentials were not accepted")]
    InvalidCredentials,
    #[error("the credentials could not be read: {0}")]
    MalformedCredentials(&'static str),
    #[error("this action requires the `{}` permission", .0.code())]
    Forbidden(Permission),
    #[error("the credentials could not be checked")]
    Unavailable,
}

#[async_trait]
pub trait AuthProvider: Send + Sync + 'static {
    async fn authenticate(&self, headers: &HeaderMap) -> Result<Principal, AuthError>;
}
