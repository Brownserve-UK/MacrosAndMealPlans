use time::OffsetDateTime;

use super::{AccessScope, HouseholdMemberId, Patch, Revision, Role, UserId, validate_name};
use crate::error::ValidationErrors;

pub const MAX_USERNAME_LEN: usize = 64;
pub const MIN_USERNAME_LEN: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HouseholdMember {
    pub id: HouseholdMemberId,
    pub display_name: String,
    pub linked_user_id: Option<UserId>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl HouseholdMember {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn has_account(&self) -> bool {
        self.linked_user_id.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct NewHouseholdMember {
    pub id: Option<HouseholdMemberId>,
    pub display_name: String,
    pub linked_user_id: Option<UserId>,
}

impl NewHouseholdMember {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name("display_name", &self.display_name, &mut errors);
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default)]
pub struct HouseholdMemberPatch {
    pub display_name: Option<String>,
}

impl HouseholdMemberPatch {
    pub fn is_empty(&self) -> bool {
        self.display_name.is_none()
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(name) = &self.display_name {
            validate_name("display_name", name, &mut errors);
        }
        errors.into_result()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: Option<String>,
    pub auth_subject: Option<String>,
    pub roles: Vec<Role>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl User {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn is_admin(&self) -> bool {
        self.roles.contains(&Role::Admin)
    }

    pub fn has(&self, permission: super::Permission) -> bool {
        self.roles
            .iter()
            .any(|role| role.permissions().contains(&permission))
    }
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub id: Option<UserId>,
    pub username: String,
    pub display_name: Option<String>,
    pub roles: Vec<Role>,
}

impl NewUser {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_username(&self.username, &mut errors);
        if let Some(display_name) = &self.display_name {
            validate_name("display_name", display_name, &mut errors);
        }
        validate_roles(&self.roles, &mut errors);
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserPatch {
    pub username: Option<String>,
    pub display_name: Patch<String>,
}

impl UserPatch {
    pub fn is_empty(&self) -> bool {
        self.username.is_none() && self.display_name.is_unchanged()
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(username) = &self.username {
            validate_username(username, &mut errors);
        }
        if let Patch::Set(display_name) = self.display_name.as_ref() {
            validate_name("display_name", display_name, &mut errors);
        }
        errors.into_result()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemberAccessGrant {
    pub grantee_user_id: UserId,
    pub subject_member_id: HouseholdMemberId,
    pub scope: AccessScope,
    pub granted_at: OffsetDateTime,
    pub granted_by: Option<UserId>,
}

fn validate_username(username: &str, errors: &mut ValidationErrors) {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        errors.push("username", "Required");
        return;
    }
    if trimmed.chars().count() < MIN_USERNAME_LEN || trimmed.chars().count() > MAX_USERNAME_LEN {
        errors.push(
            "username",
            format!("Must be {MIN_USERNAME_LEN} to {MAX_USERNAME_LEN} characters"),
        );
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        errors.push(
            "username",
            "Letters, numbers, dot, dash and underscore only",
        );
    }
}

fn validate_roles(roles: &[Role], errors: &mut ValidationErrors) {
    if roles.is_empty() {
        errors.push("roles", "Pick at least one");
    }
}

#[cfg(test)]
#[path = "household_tests.rs"]
mod tests;
