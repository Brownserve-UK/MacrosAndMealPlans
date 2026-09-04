use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    HouseholdMemberId, Revision, WeightGoal, WeightGoalId, WeightRecord, WeightRecordId,
};
use mmp_core::ports::{UpdateOutcome, WeightGoalRepository, WeightRecordRepository};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::{WeightGoalRow, WeightRecordRow};

macro_rules! record_columns {
    () => {
        "id, member_id, weight_kg, recorded_on, recorded_at, source, recorded_by, revision, created_at, updated_at"
    };
}

macro_rules! goal_columns {
    () => {
        "id, member_id, objective, starting_weight_kg, target_weight_kg, planned_rate_kg_per_week, started_on, revision, created_at, updated_at"
    };
}

const GET_RECORD: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM weight_record WHERE id = $1"
);
const LIST_RECORDS: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM weight_record WHERE member_id = $1 \
     ORDER BY recorded_on DESC, recorded_at DESC NULLS LAST, created_at DESC"
);
const RECORD_REVISION: &str = "SELECT revision FROM weight_record WHERE id = $1";

const GET_GOAL: &str = concat!(
    "SELECT ",
    goal_columns!(),
    " FROM weight_goal WHERE id = $1"
);
const GOAL_FOR_MEMBER: &str = concat!(
    "SELECT ",
    goal_columns!(),
    " FROM weight_goal WHERE member_id = $1"
);
const GOAL_REVISION: &str = "SELECT revision FROM weight_goal WHERE id = $1";

pub struct PgWeightRecordRepository {
    pool: PgPool,
}

impl PgWeightRecordRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WeightRecordRepository for PgWeightRecordRepository {
    async fn get(&self, id: WeightRecordId) -> Result<Option<WeightRecord>> {
        let row: Option<WeightRecordRow> = sqlx::query_as(GET_RECORD)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a weight record", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_for_member(&self, member_id: HouseholdMemberId) -> Result<Vec<WeightRecord>> {
        let rows: Vec<WeightRecordRow> = sqlx::query_as(LIST_RECORDS)
            .bind(member_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing weight records", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn insert(&self, record: &WeightRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO weight_record (
                 id, member_id, weight_kg, recorded_on, recorded_at, source, recorded_by,
                 revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(record.id.as_uuid())
        .bind(record.member_id.as_uuid())
        .bind(record.weight_kg)
        .bind(record.recorded_on)
        .bind(record.recorded_at)
        .bind(record.source.code())
        .bind(record.recorded_by.map(|id| id.as_uuid()))
        .bind(record.revision.get())
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "recording a weight"))?;
        Ok(())
    }

    async fn update(&self, record: &WeightRecord, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE weight_record SET
                 weight_kg = $2, recorded_on = $3, recorded_at = $4,
                 revision = $5, updated_at = $6
             WHERE id = $1 AND revision = $7",
        )
        .bind(record.id.as_uuid())
        .bind(record.weight_kg)
        .bind(record.recorded_on)
        .bind(record.recorded_at)
        .bind(record.revision.get())
        .bind(record.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a weight record"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        record_outcome(&self.pool, record.id).await
    }

    async fn delete(&self, id: WeightRecordId, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM weight_record WHERE id = $1 AND revision = $2")
            .bind(id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "deleting a weight record"))?
            .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        record_outcome(&self.pool, id).await
    }
}

pub struct PgWeightGoalRepository {
    pool: PgPool,
}

impl PgWeightGoalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WeightGoalRepository for PgWeightGoalRepository {
    async fn get(&self, id: WeightGoalId) -> Result<Option<WeightGoal>> {
        let row: Option<WeightGoalRow> = sqlx::query_as(GET_GOAL)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a weight goal", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn for_member(&self, member_id: HouseholdMemberId) -> Result<Option<WeightGoal>> {
        let row: Option<WeightGoalRow> = sqlx::query_as(GOAL_FOR_MEMBER)
            .bind(member_id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a member's weight goal", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn insert(&self, goal: &WeightGoal) -> Result<()> {
        sqlx::query(
            "INSERT INTO weight_goal (
                 id, member_id, objective, starting_weight_kg, target_weight_kg,
                 planned_rate_kg_per_week, started_on, revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(goal.id.as_uuid())
        .bind(goal.member_id.as_uuid())
        .bind(goal.objective.code())
        .bind(goal.starting_weight_kg)
        .bind(goal.target_weight_kg)
        .bind(goal.planned_rate_kg_per_week)
        .bind(goal.started_on)
        .bind(goal.revision.get())
        .bind(goal.created_at)
        .bind(goal.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "setting a weight goal"))?;
        Ok(())
    }

    async fn update(&self, goal: &WeightGoal, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE weight_goal SET
                 objective = $2, starting_weight_kg = $3, target_weight_kg = $4,
                 planned_rate_kg_per_week = $5, started_on = $6,
                 revision = $7, updated_at = $8
             WHERE id = $1 AND revision = $9",
        )
        .bind(goal.id.as_uuid())
        .bind(goal.objective.code())
        .bind(goal.starting_weight_kg)
        .bind(goal.target_weight_kg)
        .bind(goal.planned_rate_kg_per_week)
        .bind(goal.started_on)
        .bind(goal.revision.get())
        .bind(goal.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a weight goal"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        goal_outcome(&self.pool, goal.id).await
    }

    async fn delete(&self, id: WeightGoalId, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM weight_goal WHERE id = $1 AND revision = $2")
            .bind(id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "clearing a weight goal"))?
            .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        goal_outcome(&self.pool, id).await
    }
}

async fn record_outcome(pool: &PgPool, id: WeightRecordId) -> Result<UpdateOutcome> {
    let current: Option<(i64,)> = sqlx::query_as(RECORD_REVISION)
        .bind(id.as_uuid())
        .fetch_optional(pool)
        .await
        .map_err(|e| repository_error("re-reading a weight record revision", e))?;

    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}

async fn goal_outcome(pool: &PgPool, id: WeightGoalId) -> Result<UpdateOutcome> {
    let current: Option<(i64,)> = sqlx::query_as(GOAL_REVISION)
        .bind(id.as_uuid())
        .fetch_optional(pool)
        .await
        .map_err(|e| repository_error("re-reading a weight goal revision", e))?;

    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}
