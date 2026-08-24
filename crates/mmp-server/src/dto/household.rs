use mmp_core::domain::{
    AccessScope, HouseholdMember, HouseholdMemberId, HouseholdMemberPatch, MemberAccessGrant,
    NewHouseholdMember, NewUser, Patch, Role, User, UserId, UserPatch,
};
use mmp_core::ports::Paginated;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::{PageMeta, SortDirectionDto};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HouseholdMemberDto {
    pub id: Uuid,
    #[schema(example = "Joe")]
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_user_id: Option<Uuid>,
    pub has_account: bool,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub archived_at: Option<OffsetDateTime>,
}

impl From<HouseholdMember> for HouseholdMemberDto {
    fn from(value: HouseholdMember) -> Self {
        Self {
            id: value.id.as_uuid(),
            display_name: value.display_name,
            linked_user_id: value.linked_user_id.map(|id| id.as_uuid()),
            has_account: value.linked_user_id.is_some(),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserDto {
    pub id: Uuid,
    #[schema(example = "joe")]
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub roles: Vec<Role>,
    pub permissions: Vec<&'static str>,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub archived_at: Option<OffsetDateTime>,
}

impl From<User> for UserDto {
    fn from(value: User) -> Self {
        let mut permissions: Vec<&'static str> = value
            .roles
            .iter()
            .flat_map(|role| role.permissions())
            .map(|p| p.code())
            .collect();
        permissions.sort_unstable();
        permissions.dedup();

        Self {
            id: value.id.as_uuid(),
            username: value.username,
            display_name: value.display_name,
            roles: value.roles,
            permissions,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberAccessGrantDto {
    pub grantee_user_id: Uuid,
    pub subject_member_id: Uuid,
    pub scope: AccessScope,
    #[serde(with = "time::serde::rfc3339")]
    pub granted_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_by: Option<Uuid>,
}

impl From<MemberAccessGrant> for MemberAccessGrantDto {
    fn from(value: MemberAccessGrant) -> Self {
        Self {
            grantee_user_id: value.grantee_user_id.as_uuid(),
            subject_member_id: value.subject_member_id.as_uuid(),
            scope: value.scope,
            granted_at: value.granted_at,
            granted_by: value.granted_by.map(|id| id.as_uuid()),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberPage {
    pub items: Vec<HouseholdMemberDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<HouseholdMember>> for MemberPage {
    fn from(page: Paginated<HouseholdMember>) -> Self {
        let meta = PageMeta::of(&page);
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            meta,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserPage {
    pub items: Vec<UserDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<User>> for UserPage {
    fn from(page: Paginated<User>) -> Self {
        let meta = PageMeta::of(&page);
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            meta,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMemberRequest {
    #[schema(example = "Joe")]
    pub display_name: String,
    #[serde(default)]
    pub linked_user_id: Option<Uuid>,
}

impl From<CreateMemberRequest> for NewHouseholdMember {
    fn from(value: CreateMemberRequest) -> Self {
        Self {
            id: None,
            display_name: value.display_name,
            linked_user_id: value.linked_user_id.map(UserId::from),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateMemberRequest {
    #[serde(default)]
    pub display_name: Option<String>,
}

impl From<UpdateMemberRequest> for HouseholdMemberPatch {
    fn from(value: UpdateMemberRequest) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkAccountRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GrantAccessRequest {
    pub user_id: Uuid,
    pub scope: AccessScope,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    #[schema(example = "joe")]
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub roles: Vec<Role>,
}

impl From<CreateUserRequest> for NewUser {
    fn from(value: CreateUserRequest) -> Self {
        Self {
            id: None,
            username: value.username,
            display_name: value.display_name,
            roles: value.roles,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub display_name: Patch<String>,
}

impl From<UpdateUserRequest> for UserPatch {
    fn from(value: UpdateUserRequest) -> Self {
        Self {
            username: value.username,
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetRolesRequest {
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberListQuery {
    pub q: Option<String>,
    pub with_account: Option<bool>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<SortDirectionDto>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UserListQuery {
    pub q: Option<String>,
    pub role: Option<Role>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<SortDirectionDto>,
}

pub fn member_id(id: Uuid) -> HouseholdMemberId {
    HouseholdMemberId::from(id)
}

pub fn user_id(id: Uuid) -> UserId {
    UserId::from(id)
}
