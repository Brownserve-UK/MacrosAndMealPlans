use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{HouseholdMemberId, NutritionTarget, NutritionTargetId, Revision};
use mmp_core::ports::{NutritionTargetRepository, UpdateOutcome};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::NutritionTargetRow;

macro_rules! columns {
    () => {
        "id, member_id, effective_from, energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg, revision, created_at, updated_at"
    };
}

const GET_BY_ID: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM nutrition_target WHERE id = $1"
);
const LIST_FOR_MEMBER: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM nutrition_target WHERE member_id = $1 ORDER BY effective_from ASC, id ASC"
);
const CURRENT_REVISION: &str = "SELECT revision FROM nutrition_target WHERE id = $1";

pub struct PgNutritionTargetRepository {
    pool: PgPool,
}

impl PgNutritionTargetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NutritionTargetRepository for PgNutritionTargetRepository {
    async fn get(&self, id: NutritionTargetId) -> Result<Option<NutritionTarget>> {
        let row: Option<NutritionTargetRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a nutrition target", e))?;
        Ok(row.map(Into::into))
    }

    async fn list_for_member(&self, member_id: HouseholdMemberId) -> Result<Vec<NutritionTarget>> {
        let rows: Vec<NutritionTargetRow> = sqlx::query_as(LIST_FOR_MEMBER)
            .bind(member_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing nutrition targets", e))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn insert(&self, target: &NutritionTarget) -> Result<()> {
        let goals = &target.goals;
        sqlx::query(
            "INSERT INTO nutrition_target (
                 id, member_id, effective_from,
                 energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g,
                 saturated_fat_g, fibre_g, salt_g, cholesterol_mg,
                 revision, created_at, updated_at
             ) VALUES (
                 $1, $2, $3,
                 $4, $5, $6, $7, $8,
                 $9, $10, $11, $12,
                 $13, $14, $15
             )",
        )
        .bind(target.id.as_uuid())
        .bind(target.member_id.as_uuid())
        .bind(target.effective_from)
        .bind(goals.energy_kcal)
        .bind(goals.protein_g)
        .bind(goals.carbohydrate_g)
        .bind(goals.sugar_g)
        .bind(goals.fat_g)
        .bind(goals.saturated_fat_g)
        .bind(goals.fibre_g)
        .bind(goals.salt_g)
        .bind(goals.cholesterol_mg)
        .bind(target.revision.get())
        .bind(target.created_at)
        .bind(target.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a nutrition target"))?;
        Ok(())
    }

    async fn update(&self, target: &NutritionTarget, expected: Revision) -> Result<UpdateOutcome> {
        let goals = &target.goals;
        let affected = sqlx::query(
            "UPDATE nutrition_target SET
                 effective_from = $2,
                 energy_kcal = $3, protein_g = $4, carbohydrate_g = $5, sugar_g = $6,
                 fat_g = $7, saturated_fat_g = $8, fibre_g = $9, salt_g = $10,
                 cholesterol_mg = $11,
                 revision = $12, updated_at = $13
             WHERE id = $1 AND revision = $14",
        )
        .bind(target.id.as_uuid())
        .bind(target.effective_from)
        .bind(goals.energy_kcal)
        .bind(goals.protein_g)
        .bind(goals.carbohydrate_g)
        .bind(goals.sugar_g)
        .bind(goals.fat_g)
        .bind(goals.saturated_fat_g)
        .bind(goals.fibre_g)
        .bind(goals.salt_g)
        .bind(goals.cholesterol_mg)
        .bind(target.revision.get())
        .bind(target.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a nutrition target"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        outcome_from_current(&self.pool, target.id).await
    }

    async fn delete(&self, id: NutritionTargetId, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM nutrition_target WHERE id = $1 AND revision = $2")
            .bind(id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "deleting a nutrition target"))?
            .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        outcome_from_current(&self.pool, id).await
    }
}

async fn outcome_from_current(pool: &PgPool, id: NutritionTargetId) -> Result<UpdateOutcome> {
    let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
        .bind(id.as_uuid())
        .fetch_optional(pool)
        .await
        .map_err(|e| repository_error("re-reading a nutrition target revision", e))?;

    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}
