use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{HouseholdMemberId, MemberBodyProfile, Revision};
use mmp_core::ports::{MemberBodyProfileRepository, UpdateOutcome};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::MemberBodyProfileRow;

macro_rules! columns {
    () => {
        "member_id, date_of_birth, sex, height_cm, habitual_activity, revision, created_at, updated_at"
    };
}

const FOR_MEMBER: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM member_body_profile WHERE member_id = $1"
);
const CURRENT_REVISION: &str = "SELECT revision FROM member_body_profile WHERE member_id = $1";

pub struct PgMemberBodyProfileRepository {
    pool: PgPool,
}

impl PgMemberBodyProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MemberBodyProfileRepository for PgMemberBodyProfileRepository {
    async fn for_member(&self, member_id: HouseholdMemberId) -> Result<Option<MemberBodyProfile>> {
        let row: Option<MemberBodyProfileRow> = sqlx::query_as(FOR_MEMBER)
            .bind(member_id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a member body profile", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn insert(&self, profile: &MemberBodyProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO member_body_profile (
                 member_id, date_of_birth, sex, height_cm, habitual_activity,
                 revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(profile.member_id.as_uuid())
        .bind(profile.date_of_birth)
        .bind(profile.sex.map(|value| value.code()))
        .bind(profile.height_cm)
        .bind(profile.habitual_activity.map(|value| value.code()))
        .bind(profile.revision.get())
        .bind(profile.created_at)
        .bind(profile.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a member body profile"))?;
        Ok(())
    }

    async fn update(
        &self,
        profile: &MemberBodyProfile,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE member_body_profile SET
                 date_of_birth = $2, sex = $3, height_cm = $4, habitual_activity = $5,
                 revision = $6, updated_at = $7
             WHERE member_id = $1 AND revision = $8",
        )
        .bind(profile.member_id.as_uuid())
        .bind(profile.date_of_birth)
        .bind(profile.sex.map(|value| value.code()))
        .bind(profile.height_cm)
        .bind(profile.habitual_activity.map(|value| value.code()))
        .bind(profile.revision.get())
        .bind(profile.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a member body profile"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        outcome_from_current(&self.pool, profile.member_id).await
    }
}

async fn outcome_from_current(
    pool: &PgPool,
    member_id: HouseholdMemberId,
) -> Result<UpdateOutcome> {
    let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
        .bind(member_id.as_uuid())
        .fetch_optional(pool)
        .await
        .map_err(|e| repository_error("re-reading a member body profile revision", e))?;
    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}
