use std::collections::HashMap;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    MealPlanComponentId, NewStockEvent, PreparedBatch, PreparedBatchId, StockItem, StockOutcome,
};
use mmp_core::ports::{PreparedBatchRepository, StockWrite};
use sqlx::PgPool;
use time::Date;
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{PreparedBatchRow, nutrition_bindings};
use crate::stock::{apply_stock_write, insert_stock_item};

macro_rules! columns {
    () => {
        "id, recipe_id, meal_plan_entry_id, meal_plan_component_id, prepared_at, \
         servings_produced, frozen_item_name, nutrition_basis_amount, nutrition_basis_unit, \
         energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, \
         salt_g, cholesterol_mg, nutrition_extra, nutrition_quality, created_by, revision, \
         created_at, updated_at"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM prepared_batch WHERE id = $1");
const GET_MANY: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM prepared_batch WHERE id = ANY($1)"
);
const FOR_COMPONENT: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM prepared_batch WHERE meal_plan_component_id = $1 \
     ORDER BY prepared_at ASC, id ASC LIMIT 1"
);
const FOR_COMPONENTS: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM prepared_batch WHERE meal_plan_component_id = ANY($1) \
     ORDER BY prepared_at ASC, id ASC"
);
const LIST_IN_RANGE: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM prepared_batch WHERE (prepared_at AT TIME ZONE 'UTC')::date BETWEEN $1 AND $2 \
     ORDER BY prepared_at DESC, id DESC"
);

pub struct PgPreparedBatchRepository {
    pool: PgPool,
}

impl PgPreparedBatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PreparedBatchRepository for PgPreparedBatchRepository {
    async fn get(&self, id: PreparedBatchId) -> Result<Option<PreparedBatch>> {
        let row: Option<PreparedBatchRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a prepared batch", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn get_many(&self, ids: &[PreparedBatchId]) -> Result<Vec<PreparedBatch>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let uuids: Vec<Uuid> = ids.iter().map(|id| id.as_uuid()).collect();
        let rows: Vec<PreparedBatchRow> = sqlx::query_as(GET_MANY)
            .bind(&uuids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("loading prepared batches", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn for_component(
        &self,
        component_id: MealPlanComponentId,
    ) -> Result<Option<PreparedBatch>> {
        let row: Option<PreparedBatchRow> = sqlx::query_as(FOR_COMPONENT)
            .bind(component_id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a component's prepared batch", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn for_components(
        &self,
        component_ids: &[MealPlanComponentId],
    ) -> Result<HashMap<MealPlanComponentId, PreparedBatch>> {
        if component_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let uuids: Vec<Uuid> = component_ids.iter().map(|id| id.as_uuid()).collect();
        let rows: Vec<PreparedBatchRow> = sqlx::query_as(FOR_COMPONENTS)
            .bind(&uuids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("loading prepared batches for components", e))?;
        let mut found = HashMap::new();
        for row in rows {
            let batch: PreparedBatch = row.try_into()?;
            if let Some(component_id) = batch.source.component_id() {
                found.entry(component_id).or_insert(batch);
            }
        }
        Ok(found)
    }

    async fn list_in_range(&self, from: Date, to: Date) -> Result<Vec<PreparedBatch>> {
        let rows: Vec<PreparedBatchRow> = sqlx::query_as(LIST_IN_RANGE)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing prepared batches", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn insert(
        &self,
        batch: &PreparedBatch,
        portions: &[(StockItem, NewStockEvent)],
        stock: &StockWrite,
    ) -> Result<Vec<StockOutcome>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a preparation", e))?;

        let nutrition = nutrition_bindings(&batch.nutrition.facts);
        sqlx::query(
            "INSERT INTO prepared_batch (
                 id, recipe_id, meal_plan_entry_id, meal_plan_component_id, prepared_at,
                 servings_produced, frozen_item_name, nutrition_basis_amount, nutrition_basis_unit,
                 energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g,
                 fibre_g, salt_g, cholesterol_mg, nutrition_extra, nutrition_quality,
                 created_by, revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                       $17, $18, $19, $20, $21, $22, $23, $24)",
        )
        .bind(batch.id.as_uuid())
        .bind(batch.recipe_id.map(|id| id.as_uuid()))
        .bind(batch.source.entry_id().map(|id| id.as_uuid()))
        .bind(batch.source.component_id().map(|id| id.as_uuid()))
        .bind(batch.prepared_at)
        .bind(batch.servings_produced)
        .bind(&batch.item_name)
        .bind(nutrition.basis_amount)
        .bind(nutrition.basis_unit)
        .bind(nutrition.energy_kcal)
        .bind(nutrition.protein_g)
        .bind(nutrition.carbohydrate_g)
        .bind(nutrition.sugar_g)
        .bind(nutrition.fat_g)
        .bind(nutrition.saturated_fat_g)
        .bind(nutrition.fibre_g)
        .bind(nutrition.salt_g)
        .bind(nutrition.cholesterol_mg)
        .bind(nutrition.extra)
        .bind(batch.nutrition.quality.code())
        .bind(batch.created_by.as_uuid())
        .bind(batch.revision.get())
        .bind(batch.created_at)
        .bind(batch.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "recording a preparation"))?;

        for (portion, event) in portions {
            insert_stock_item(&mut tx, portion, event).await?;
        }
        let outcomes = apply_stock_write(&mut tx, stock, batch.prepared_at).await?;

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a preparation", e))?;
        Ok(outcomes)
    }
}
