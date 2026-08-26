use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{ConsumptionRecord, ConsumptionRecordId, HouseholdMemberId, Revision};
use mmp_core::ports::{
    ConsumptionQuery, ConsumptionRecordRepository, Paginated, SortDirection, UpdateOutcome,
};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::{ConsumptionRecordRow, amount_bindings, nutrition_bindings};

macro_rules! columns {
    () => {
        "c.id, c.member_id, c.product_id, c.recorded_by, mpc.entry_id AS meal_plan_entry_id, c.meal_plan_component_id, c.slot, c.amount_kind, c.amount_value, c.amount_unit, c.consumed_on, c.consumed_at, c.nutrition_basis_amount, c.nutrition_basis_unit, c.energy_kcal, c.protein_g, c.carbohydrate_g, c.sugar_g, c.fat_g, c.saturated_fat_g, c.fibre_g, c.salt_g, c.cholesterol_mg, c.nutrition_extra, c.nutrition_quality, c.revision, c.created_at, c.updated_at"
    };
}

macro_rules! filter {
    () => {
        " WHERE ($1::uuid IS NULL OR c.member_id = $1) \
          AND ($2::date IS NULL OR c.consumed_on >= $2) \
          AND ($3::date IS NULL OR c.consumed_on <= $3)"
    };
}

const GET_BY_ID: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM consumption_record c LEFT JOIN meal_plan_component mpc ON mpc.id = c.meal_plan_component_id WHERE c.id = $1"
);
const COUNT: &str = concat!("SELECT count(*) FROM consumption_record c", filter!());
const LIST_ASC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM consumption_record c LEFT JOIN meal_plan_component mpc ON mpc.id = c.meal_plan_component_id",
    filter!(),
    " ORDER BY c.consumed_at ASC, c.id ASC LIMIT $4 OFFSET $5"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM consumption_record c LEFT JOIN meal_plan_component mpc ON mpc.id = c.meal_plan_component_id",
    filter!(),
    " ORDER BY c.consumed_at DESC, c.id DESC LIMIT $4 OFFSET $5"
);
const LIST_PERIOD: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM consumption_record c LEFT JOIN meal_plan_component mpc ON mpc.id = c.meal_plan_component_id ",
    "WHERE c.member_id = $1 AND c.consumed_on >= $2 AND c.consumed_on <= $3 ",
    "ORDER BY c.consumed_at ASC, c.id ASC"
);
const LIST_FOR_MEAL_PLAN_ENTRY: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM consumption_record c JOIN meal_plan_component mpc ON mpc.id = c.meal_plan_component_id ",
    "WHERE mpc.entry_id = $1 ORDER BY c.consumed_at ASC, c.id ASC"
);
const CURRENT_REVISION: &str = "SELECT revision FROM consumption_record WHERE id = $1";
const DELETE: &str = "DELETE FROM consumption_record WHERE id = $1";

pub struct PgConsumptionRecordRepository {
    pool: PgPool,
}

impl PgConsumptionRecordRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConsumptionRecordRepository for PgConsumptionRecordRepository {
    async fn get(&self, id: ConsumptionRecordId) -> Result<Option<ConsumptionRecord>> {
        let row: Option<ConsumptionRecordRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a consumption record", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &ConsumptionQuery) -> Result<Paginated<ConsumptionRecord>> {
        let member_id = query.member_id.map(|id| id.as_uuid());
        let list_sql = match query.sort {
            SortDirection::Ascending => LIST_ASC,
            SortDirection::Descending => LIST_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(member_id)
            .bind(query.from)
            .bind(query.to)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting consumption records", e))?;

        let rows: Vec<ConsumptionRecordRow> = sqlx::query_as(list_sql)
            .bind(member_id)
            .bind(query.from)
            .bind(query.to)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing consumption records", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<ConsumptionRecord>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn list_period(
        &self,
        member_id: HouseholdMemberId,
        from: time::Date,
        to: time::Date,
    ) -> Result<Vec<ConsumptionRecord>> {
        let rows: Vec<ConsumptionRecordRow> = sqlx::query_as(LIST_PERIOD)
            .bind(member_id.as_uuid())
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing consumption records for a period", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn list_for_meal_plan_entry(
        &self,
        entry_id: mmp_core::domain::MealPlanEntryId,
    ) -> Result<Vec<ConsumptionRecord>> {
        let rows: Vec<ConsumptionRecordRow> = sqlx::query_as(LIST_FOR_MEAL_PLAN_ENTRY)
            .bind(entry_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing consumption records for a meal plan", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn insert(&self, record: &ConsumptionRecord) -> Result<()> {
        let (amount_kind, amount_value, amount_unit) = amount_bindings(&record.amount);
        let n = nutrition_bindings(&record.nutrition);
        sqlx::query(
            "INSERT INTO consumption_record (
                 id, member_id, product_id, recorded_by, meal_plan_component_id, slot,
                 amount_kind, amount_value, amount_unit,
                 consumed_on, consumed_at,
                 nutrition_basis_amount, nutrition_basis_unit,
                 energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g,
                 saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra,
                 nutrition_quality,
                 revision, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5, $6,
                 $7, $8, $9,
                 $10, $11,
                 $12, $13,
                 $14, $15, $16, $17, $18,
                 $19, $20, $21, $22, $23,
                 $24,
                 $25, $26, $27
             )",
        )
        .bind(record.id.as_uuid())
        .bind(record.member_id.as_uuid())
        .bind(record.product_id.as_uuid())
        .bind(record.recorded_by.map(|id| id.as_uuid()))
        .bind(record.meal_plan_component_id.map(|id| id.as_uuid()))
        .bind(record.slot.code())
        .bind(amount_kind)
        .bind(amount_value)
        .bind(amount_unit)
        .bind(record.consumed_on)
        .bind(record.consumed_at)
        .bind(n.basis_amount)
        .bind(n.basis_unit)
        .bind(n.energy_kcal)
        .bind(n.protein_g)
        .bind(n.carbohydrate_g)
        .bind(n.sugar_g)
        .bind(n.fat_g)
        .bind(n.saturated_fat_g)
        .bind(n.fibre_g)
        .bind(n.salt_g)
        .bind(n.cholesterol_mg)
        .bind(n.extra)
        .bind(record.quality.code())
        .bind(record.revision.get())
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a consumption record"))?;
        Ok(())
    }

    async fn update(
        &self,
        record: &ConsumptionRecord,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let (amount_kind, amount_value, amount_unit) = amount_bindings(&record.amount);
        let n = nutrition_bindings(&record.nutrition);
        let affected = sqlx::query(
            "UPDATE consumption_record SET
                 member_id = $2, product_id = $3, recorded_by = $4, slot = $5,
                 amount_kind = $6, amount_value = $7, amount_unit = $8,
                 consumed_on = $9, consumed_at = $10,
                 nutrition_basis_amount = $11, nutrition_basis_unit = $12,
                 energy_kcal = $13, protein_g = $14, carbohydrate_g = $15,
                 sugar_g = $16, fat_g = $17, saturated_fat_g = $18, fibre_g = $19,
                 salt_g = $20, cholesterol_mg = $21, nutrition_extra = $22,
                 nutrition_quality = $23,
                 revision = $24, updated_at = $25
             WHERE id = $1 AND revision = $26",
        )
        .bind(record.id.as_uuid())
        .bind(record.member_id.as_uuid())
        .bind(record.product_id.as_uuid())
        .bind(record.recorded_by.map(|id| id.as_uuid()))
        .bind(record.slot.code())
        .bind(amount_kind)
        .bind(amount_value)
        .bind(amount_unit)
        .bind(record.consumed_on)
        .bind(record.consumed_at)
        .bind(n.basis_amount)
        .bind(n.basis_unit)
        .bind(n.energy_kcal)
        .bind(n.protein_g)
        .bind(n.carbohydrate_g)
        .bind(n.sugar_g)
        .bind(n.fat_g)
        .bind(n.saturated_fat_g)
        .bind(n.fibre_g)
        .bind(n.salt_g)
        .bind(n.cholesterol_mg)
        .bind(n.extra)
        .bind(record.quality.code())
        .bind(record.revision.get())
        .bind(record.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a consumption record"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(record.id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("re-reading a consumption record revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }

    async fn delete(&self, id: ConsumptionRecordId) -> Result<bool> {
        let affected = sqlx::query(DELETE)
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "deleting a consumption record"))?
            .rows_affected();
        Ok(affected == 1)
    }
}
