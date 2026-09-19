use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{CalorieCalculation, CalorieCalculationId, NutritionTargetId, Revision};
use mmp_core::ports::{CalorieCalculationRepository, UpdateOutcome};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::CalorieCalculationRow;

macro_rules! columns {
    () => {
        "id, member_id, nutrition_target_id, calculated_on, formula, activity_source, habitual_activity, age_years, sex, height_cm, weight_kg, objective, emphasis, requested_rate_kg_per_week, applied_rate_kg_per_week, maintenance_kcal, adjustment_kcal, recommended_kcal, floor_kcal, eased, revision, created_at, updated_at"
    };
}

const GET_BY_ID: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM calorie_target_calculation WHERE id = $1"
);
const FOR_TARGET: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM calorie_target_calculation WHERE nutrition_target_id = $1"
);
const CURRENT_REVISION: &str = "SELECT revision FROM calorie_target_calculation WHERE id = $1";

pub struct PgCalorieCalculationRepository {
    pool: PgPool,
}

impl PgCalorieCalculationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CalorieCalculationRepository for PgCalorieCalculationRepository {
    async fn get(&self, id: CalorieCalculationId) -> Result<Option<CalorieCalculation>> {
        let row: Option<CalorieCalculationRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a calorie target calculation", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn for_target(&self, target_id: NutritionTargetId) -> Result<Option<CalorieCalculation>> {
        let row: Option<CalorieCalculationRow> = sqlx::query_as(FOR_TARGET)
            .bind(target_id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a target's calorie calculation", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn insert(&self, calculation: &CalorieCalculation) -> Result<()> {
        sqlx::query(
            "INSERT INTO calorie_target_calculation (
                 id, member_id, nutrition_target_id, calculated_on, formula, activity_source,
                 habitual_activity, age_years, sex, height_cm, weight_kg, objective, emphasis,
                 requested_rate_kg_per_week, applied_rate_kg_per_week, maintenance_kcal,
                 adjustment_kcal, recommended_kcal, floor_kcal, eased,
                 revision, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                 $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23
             )",
        )
        .bind(calculation.id.as_uuid())
        .bind(calculation.member_id.as_uuid())
        .bind(calculation.nutrition_target_id.as_uuid())
        .bind(calculation.calculated_on)
        .bind(&calculation.formula)
        .bind(&calculation.activity_source)
        .bind(calculation.habitual_activity.code())
        .bind(calculation.age_years)
        .bind(calculation.sex.code())
        .bind(calculation.height_cm)
        .bind(calculation.weight_kg)
        .bind(calculation.objective.code())
        .bind(calculation.emphasis.code())
        .bind(calculation.requested_rate_kg_per_week)
        .bind(calculation.applied_rate_kg_per_week)
        .bind(calculation.maintenance_kcal)
        .bind(calculation.adjustment_kcal)
        .bind(calculation.recommended_kcal)
        .bind(calculation.floor_kcal)
        .bind(calculation.eased)
        .bind(calculation.revision.get())
        .bind(calculation.created_at)
        .bind(calculation.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a calorie target calculation"))?;
        Ok(())
    }

    async fn update(
        &self,
        calculation: &CalorieCalculation,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE calorie_target_calculation SET
                 member_id = $2, nutrition_target_id = $3, calculated_on = $4, formula = $5,
                 activity_source = $6, habitual_activity = $7, age_years = $8, sex = $9,
                 height_cm = $10, weight_kg = $11, objective = $12, emphasis = $13,
                 requested_rate_kg_per_week = $14, applied_rate_kg_per_week = $15,
                 maintenance_kcal = $16, adjustment_kcal = $17, recommended_kcal = $18,
                 floor_kcal = $19, eased = $20, revision = $21, updated_at = $22
             WHERE id = $1 AND revision = $23",
        )
        .bind(calculation.id.as_uuid())
        .bind(calculation.member_id.as_uuid())
        .bind(calculation.nutrition_target_id.as_uuid())
        .bind(calculation.calculated_on)
        .bind(&calculation.formula)
        .bind(&calculation.activity_source)
        .bind(calculation.habitual_activity.code())
        .bind(calculation.age_years)
        .bind(calculation.sex.code())
        .bind(calculation.height_cm)
        .bind(calculation.weight_kg)
        .bind(calculation.objective.code())
        .bind(calculation.emphasis.code())
        .bind(calculation.requested_rate_kg_per_week)
        .bind(calculation.applied_rate_kg_per_week)
        .bind(calculation.maintenance_kcal)
        .bind(calculation.adjustment_kcal)
        .bind(calculation.recommended_kcal)
        .bind(calculation.floor_kcal)
        .bind(calculation.eased)
        .bind(calculation.revision.get())
        .bind(calculation.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a calorie target calculation"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        outcome_from_current(&self.pool, calculation.id).await
    }

    async fn delete(&self, id: CalorieCalculationId, expected: Revision) -> Result<UpdateOutcome> {
        let affected =
            sqlx::query("DELETE FROM calorie_target_calculation WHERE id = $1 AND revision = $2")
                .bind(id.as_uuid())
                .bind(expected.get())
                .execute(&self.pool)
                .await
                .map_err(|e| map_db_error(e, "deleting a calorie target calculation"))?
                .rows_affected();
        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        outcome_from_current(&self.pool, id).await
    }
}

async fn outcome_from_current(pool: &PgPool, id: CalorieCalculationId) -> Result<UpdateOutcome> {
    let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
        .bind(id.as_uuid())
        .fetch_optional(pool)
        .await
        .map_err(|e| repository_error("re-reading a calorie calculation revision", e))?;
    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}
