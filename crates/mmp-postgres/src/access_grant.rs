use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{AccessScope, HouseholdMemberId, MemberAccessGrant, UserId};
use mmp_core::ports::AccessGrantRepository;
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::MemberAccessGrantRow;

const LIST_FOR_MEMBER: &str = "SELECT grantee_user_id, subject_member_id, scope, granted_at, granted_by \
     FROM member_access_grant WHERE subject_member_id = $1 ORDER BY scope, grantee_user_id";
const UPSERT: &str = "INSERT INTO member_access_grant \
     (grantee_user_id, subject_member_id, scope, granted_at, granted_by) \
     VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (grantee_user_id, subject_member_id, scope) \
     DO UPDATE SET granted_at = EXCLUDED.granted_at, granted_by = EXCLUDED.granted_by";
const EXISTS: &str = "SELECT EXISTS (SELECT 1 FROM member_access_grant \
     WHERE grantee_user_id = $1 AND subject_member_id = $2 AND scope = $3)";
const REVOKE: &str = "DELETE FROM member_access_grant \
     WHERE grantee_user_id = $1 AND subject_member_id = $2 AND scope = $3";

pub struct PgAccessGrantRepository {
    pool: PgPool,
}

impl PgAccessGrantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AccessGrantRepository for PgAccessGrantRepository {
    async fn list_for_member(
        &self,
        member_id: HouseholdMemberId,
    ) -> Result<Vec<MemberAccessGrant>> {
        let rows: Vec<MemberAccessGrantRow> = sqlx::query_as(LIST_FOR_MEMBER)
            .bind(member_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing access grants", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn exists(
        &self,
        grantee_user_id: UserId,
        subject_member_id: HouseholdMemberId,
        scope: AccessScope,
    ) -> Result<bool> {
        let found: (bool,) = sqlx::query_as(EXISTS)
            .bind(grantee_user_id.as_uuid())
            .bind(subject_member_id.as_uuid())
            .bind(scope.code())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("checking an access grant", e))?;
        Ok(found.0)
    }

    async fn upsert(&self, grant: &MemberAccessGrant) -> Result<()> {
        sqlx::query(UPSERT)
            .bind(grant.grantee_user_id.as_uuid())
            .bind(grant.subject_member_id.as_uuid())
            .bind(grant.scope.code())
            .bind(grant.granted_at)
            .bind(grant.granted_by.map(|id| id.as_uuid()))
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "granting access"))?;
        Ok(())
    }

    async fn revoke(
        &self,
        grantee_user_id: UserId,
        subject_member_id: HouseholdMemberId,
        scope: AccessScope,
    ) -> Result<bool> {
        let affected = sqlx::query(REVOKE)
            .bind(grantee_user_id.as_uuid())
            .bind(subject_member_id.as_uuid())
            .bind(scope.code())
            .execute(&self.pool)
            .await
            .map_err(|e| repository_error("revoking access", e))?
            .rows_affected();
        Ok(affected > 0)
    }
}
