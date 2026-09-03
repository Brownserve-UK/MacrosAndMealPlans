use std::sync::Arc;

use time::OffsetDateTime;

use crate::domain::{
    AccessScope, HouseholdMember, HouseholdMemberId, HouseholdMemberPatch, MemberAccessGrant,
    NewHouseholdMember, NewUser, Permission, Revision, Role, User, UserId, UserPatch,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    AccessGrantRepository, Clock, HouseholdMemberRepository, MemberQuery, Paginated, UpdateOutcome,
    UserQuery, UserRepository,
};

const MEMBER: &str = "household member";
const USER: &str = "user";

#[derive(Clone)]
pub struct HouseholdService {
    members: Arc<dyn HouseholdMemberRepository>,
    users: Arc<dyn UserRepository>,
    grants: Arc<dyn AccessGrantRepository>,
    clock: Arc<dyn Clock>,
}

impl HouseholdService {
    pub fn new(
        members: Arc<dyn HouseholdMemberRepository>,
        users: Arc<dyn UserRepository>,
        grants: Arc<dyn AccessGrantRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            members,
            users,
            grants,
            clock,
        }
    }

    pub async fn create_member(&self, input: NewHouseholdMember) -> Result<HouseholdMember> {
        input.validate()?;
        let display_name = input.display_name.trim().to_owned();
        self.ensure_member_name_free(&display_name, None).await?;

        if let Some(user_id) = input.linked_user_id {
            self.ensure_linkable(user_id, None).await?;
        }

        let now = self.clock.now();
        let member = HouseholdMember {
            id: input.id.unwrap_or_default(),
            display_name,
            linked_user_id: input.linked_user_id,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        self.members.insert(&member).await?;
        Ok(member)
    }

    pub async fn get_member(&self, id: HouseholdMemberId) -> Result<HouseholdMember> {
        self.members
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEMBER, id))
    }

    pub async fn find_member_by_linked_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<HouseholdMember>> {
        self.members.find_by_linked_user(user_id).await
    }

    pub async fn list_members(&self, query: &MemberQuery) -> Result<Paginated<HouseholdMember>> {
        self.members.list(query).await
    }

    pub async fn update_member(
        &self,
        id: HouseholdMemberId,
        expected: Revision,
        patch: HouseholdMemberPatch,
    ) -> Result<HouseholdMember> {
        patch.validate()?;
        let mut current = self.get_member(id).await?;
        require_revision(MEMBER, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(display_name) = patch.display_name {
            let display_name = display_name.trim().to_owned();
            if !display_name.eq_ignore_ascii_case(&current.display_name) {
                self.ensure_member_name_free(&display_name, Some(id))
                    .await?;
            }
            current.display_name = display_name;
        }

        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_member(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_member_archived(
        &self,
        id: HouseholdMemberId,
        expected: Revision,
        archived: bool,
    ) -> Result<HouseholdMember> {
        let mut current = self.get_member(id).await?;
        require_revision(MEMBER, id, expected, current.revision)?;

        if current.is_archived() == archived {
            return Ok(current);
        }

        current.archived_at = archived.then(|| self.clock.now());
        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_member(&current, expected).await?;
        Ok(current)
    }

    pub async fn link_account(
        &self,
        id: HouseholdMemberId,
        expected: Revision,
        user_id: UserId,
    ) -> Result<HouseholdMember> {
        let mut current = self.get_member(id).await?;
        require_revision(MEMBER, id, expected, current.revision)?;

        if current.linked_user_id == Some(user_id) {
            return Ok(current);
        }

        self.ensure_linkable(user_id, Some(id)).await?;

        current.linked_user_id = Some(user_id);
        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_member(&current, expected).await?;
        Ok(current)
    }

    pub async fn unlink_account(
        &self,
        id: HouseholdMemberId,
        expected: Revision,
    ) -> Result<HouseholdMember> {
        let mut current = self.get_member(id).await?;
        require_revision(MEMBER, id, expected, current.revision)?;

        if current.linked_user_id.is_none() {
            return Ok(current);
        }

        current.linked_user_id = None;
        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_member(&current, expected).await?;
        Ok(current)
    }

    pub async fn create_user(&self, input: NewUser) -> Result<User> {
        input.validate()?;
        let username = input.username.trim().to_owned();
        self.ensure_username_free(&username, None).await?;

        let now = self.clock.now();
        let user = User {
            id: input.id.unwrap_or_default(),
            username,
            display_name: normalise_optional(input.display_name),
            auth_subject: None,
            roles: normalise_roles(input.roles),
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        self.users.insert(&user).await?;
        Ok(user)
    }

    pub async fn get_user(&self, id: UserId) -> Result<User> {
        self.users
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(USER, id))
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        self.users.find_by_username(username).await
    }

    pub async fn list_users(&self, query: &UserQuery) -> Result<Paginated<User>> {
        self.users.list(query).await
    }

    pub async fn update_user(
        &self,
        id: UserId,
        expected: Revision,
        patch: UserPatch,
    ) -> Result<User> {
        patch.validate()?;
        let mut current = self.get_user(id).await?;
        require_revision(USER, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(username) = patch.username {
            let username = username.trim().to_owned();
            if !username.eq_ignore_ascii_case(&current.username) {
                self.ensure_username_free(&username, Some(id)).await?;
            }
            current.username = username;
        }
        current.display_name = patch
            .display_name
            .map(|v| v.trim().to_owned())
            .apply(current.display_name);

        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_user(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_user_roles(
        &self,
        id: UserId,
        expected: Revision,
        roles: Vec<Role>,
    ) -> Result<User> {
        let mut current = self.get_user(id).await?;
        require_revision(USER, id, expected, current.revision)?;

        let roles = normalise_roles(roles);
        if roles.is_empty() {
            let mut errors = ValidationErrors::new();
            errors.push("roles", "Pick at least one");
            errors.into_result()?;
        }

        if current.roles == roles {
            return Ok(current);
        }

        if current.is_admin() && !roles.contains(&Role::Admin) {
            self.ensure_another_admin_remains(id).await?;
        }

        current.roles = roles;
        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_user(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_user_archived(
        &self,
        id: UserId,
        expected: Revision,
        archived: bool,
    ) -> Result<User> {
        let mut current = self.get_user(id).await?;
        require_revision(USER, id, expected, current.revision)?;

        if current.is_archived() == archived {
            return Ok(current);
        }

        if archived && current.is_admin() {
            self.ensure_another_admin_remains(id).await?;
        }

        current.archived_at = archived.then(|| self.clock.now());
        self.stamp(&mut current.revision, &mut current.updated_at);
        self.commit_user(&current, expected).await?;
        Ok(current)
    }

    pub async fn list_member_access(
        &self,
        member_id: HouseholdMemberId,
    ) -> Result<Vec<MemberAccessGrant>> {
        self.get_member(member_id).await?;
        self.grants.list_for_member(member_id).await
    }

    pub async fn grant_access(
        &self,
        member_id: HouseholdMemberId,
        grantee_user_id: UserId,
        scope: AccessScope,
        granted_by: Option<UserId>,
    ) -> Result<MemberAccessGrant> {
        self.get_member(member_id).await?;
        self.get_user(grantee_user_id).await?;

        let grant = MemberAccessGrant {
            grantee_user_id,
            subject_member_id: member_id,
            scope,
            granted_at: self.clock.now(),
            granted_by,
        };
        self.grants.upsert(&grant).await?;
        Ok(grant)
    }

    pub async fn revoke_access(
        &self,
        member_id: HouseholdMemberId,
        grantee_user_id: UserId,
        scope: AccessScope,
    ) -> Result<()> {
        if !self
            .grants
            .revoke(grantee_user_id, member_id, scope)
            .await?
        {
            return Err(CoreError::not_found(
                "access grant",
                format!("{grantee_user_id}/{member_id}/{scope}"),
            ));
        }
        Ok(())
    }

    pub async fn can_view_member_health_data(
        &self,
        user: &User,
        member_id: HouseholdMemberId,
    ) -> Result<bool> {
        if user.has(Permission::MemberHealthData) {
            return Ok(true);
        }

        let member = self.get_member(member_id).await?;
        if member.linked_user_id == Some(user.id) {
            return Ok(true);
        }

        self.grants
            .exists(user.id, member_id, AccessScope::HealthData)
            .await
    }

    pub async fn can_manage_member_meal_plan(
        &self,
        user_id: UserId,
        member_id: HouseholdMemberId,
    ) -> Result<bool> {
        let member = self.get_member(member_id).await?;
        if member.linked_user_id == Some(user_id) {
            return Ok(true);
        }
        self.grants
            .exists(user_id, member_id, AccessScope::MealPlan)
            .await
    }

    async fn commit_member(&self, member: &HouseholdMember, expected: Revision) -> Result<()> {
        match self.members.update(member, expected).await? {
            UpdateOutcome::Updated => Ok(()),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: MEMBER,
                id: member.id.to_string(),
                expected,
                actual,
            }),
            UpdateOutcome::NotFound => Err(CoreError::not_found(MEMBER, member.id)),
        }
    }

    async fn commit_user(&self, user: &User, expected: Revision) -> Result<()> {
        match self.users.update(user, expected).await? {
            UpdateOutcome::Updated => Ok(()),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: USER,
                id: user.id.to_string(),
                expected,
                actual,
            }),
            UpdateOutcome::NotFound => Err(CoreError::not_found(USER, user.id)),
        }
    }

    fn stamp(&self, revision: &mut Revision, updated_at: &mut OffsetDateTime) {
        *revision = revision.next();
        *updated_at = self.clock.now();
    }

    async fn ensure_member_name_free(
        &self,
        name: &str,
        allow: Option<HouseholdMemberId>,
    ) -> Result<()> {
        if let Some(existing) = self.members.find_by_display_name(name).await?
            && Some(existing.id) != allow
        {
            return Err(CoreError::duplicate(MEMBER, "name", name));
        }
        Ok(())
    }

    async fn ensure_username_free(&self, username: &str, allow: Option<UserId>) -> Result<()> {
        if let Some(existing) = self.users.find_by_username(username).await?
            && Some(existing.id) != allow
        {
            return Err(CoreError::duplicate(USER, "username", username));
        }
        Ok(())
    }

    async fn ensure_linkable(
        &self,
        user_id: UserId,
        allow: Option<HouseholdMemberId>,
    ) -> Result<()> {
        let user = self.get_user(user_id).await?;
        let mut errors = ValidationErrors::new();

        if user.is_archived() {
            errors.push("linked_user_id", "That account is archived");
        }

        if let Some(existing) = self.members.find_by_linked_user(user_id).await?
            && Some(existing.id) != allow
        {
            errors.push(
                "linked_user_id",
                format!("{} already uses that account", existing.display_name),
            );
        }

        errors.into_result()
    }

    async fn ensure_another_admin_remains(&self, excluding: UserId) -> Result<()> {
        let admins = self.users.count_with_role(Role::Admin, false).await?;
        let excluded = match self.users.get(excluding).await? {
            Some(user) => i64::from(user.is_admin() && !user.is_archived()),
            None => 0,
        };

        if admins - excluded < 1 {
            let mut errors = ValidationErrors::new();
            errors.push("roles", "The last admin has to stay an admin");
            return errors.into_result();
        }
        Ok(())
    }
}

fn require_revision(
    resource: &'static str,
    id: impl std::fmt::Display,
    expected: Revision,
    actual: Revision,
) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn normalise_optional(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn normalise_roles(mut roles: Vec<Role>) -> Vec<Role> {
    roles.sort_unstable();
    roles.dedup();
    roles
}

#[cfg(test)]
#[path = "household_tests.rs"]
mod tests;
