use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    ConsumptionRecord, MealPlanComponent, MealPlanComponentId, MealPlanComponentSnapshot,
    MealPlanEntry, MealPlanEntryId, MealPlanStatus, MealSlot, NutritionFacts, NutritionQuality,
    Revision, UserId,
};
use mmp_core::ports::{MealPlanQuery, MealPlanRepository, UpdateOutcome};
use rust_decimal::Decimal;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{amount_bindings, bad_value, nutrition_bindings, parse_amount, parse_basis};

type Extra = Json<BTreeMap<String, Decimal>>;

#[derive(Debug, sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    member_id: Uuid,
    planned_on: Date,
    planned_time: Option<Time>,
    slot: String,
    status: String,
    created_by: Uuid,
    updated_by: Uuid,
    resolved_by: Option<Uuid>,
    resolved_at: Option<OffsetDateTime>,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct ComponentRow {
    id: Uuid,
    entry_id: Uuid,
    position: i32,
    product_id: Uuid,
    amount_kind: String,
    amount_value: Decimal,
    amount_unit: Option<String>,
    frozen_product_name: Option<String>,
    nutrition_basis_amount: Option<Decimal>,
    nutrition_basis_unit: Option<String>,
    energy_kcal: Option<Decimal>,
    protein_g: Option<Decimal>,
    carbohydrate_g: Option<Decimal>,
    sugar_g: Option<Decimal>,
    fat_g: Option<Decimal>,
    saturated_fat_g: Option<Decimal>,
    fibre_g: Option<Decimal>,
    salt_g: Option<Decimal>,
    cholesterol_mg: Option<Decimal>,
    nutrition_extra: Option<Extra>,
    nutrition_quality: Option<String>,
}

impl ComponentRow {
    fn into_domain(self) -> Result<MealPlanComponent> {
        let snapshot = match (self.frozen_product_name, self.nutrition_quality) {
            (Some(product_name), Some(quality)) => Some(MealPlanComponentSnapshot {
                product_name,
                nutrition: NutritionFacts {
                    basis: parse_basis(self.nutrition_basis_amount, self.nutrition_basis_unit)?,
                    energy_kcal: self.energy_kcal,
                    protein_g: self.protein_g,
                    carbohydrate_g: self.carbohydrate_g,
                    sugar_g: self.sugar_g,
                    fat_g: self.fat_g,
                    saturated_fat_g: self.saturated_fat_g,
                    fibre_g: self.fibre_g,
                    salt_g: self.salt_g,
                    cholesterol_mg: self.cholesterol_mg,
                    extra: self
                        .nutrition_extra
                        .map(|value| value.0)
                        .unwrap_or_default(),
                },
                quality: NutritionQuality::from_str(&quality)
                    .map_err(|_| bad_value("nutrition_quality", &quality))?,
            }),
            (None, None) => None,
            _ => return Err(bad_value("meal_plan_component_snapshot", "partial")),
        };
        Ok(MealPlanComponent {
            id: MealPlanComponentId::from(self.id),
            product_id: self.product_id.into(),
            amount: parse_amount(&self.amount_kind, self.amount_value, self.amount_unit)?,
            position: self.position,
            snapshot,
        })
    }
}

fn assemble(row: EntryRow, components: Vec<MealPlanComponent>) -> Result<MealPlanEntry> {
    Ok(MealPlanEntry {
        id: MealPlanEntryId::from(row.id),
        member_id: row.member_id.into(),
        planned_on: row.planned_on,
        planned_time: row.planned_time,
        slot: MealSlot::from_str(&row.slot).map_err(|_| bad_value("slot", &row.slot))?,
        status: MealPlanStatus::from_str(&row.status)
            .map_err(|_| bad_value("status", &row.status))?,
        components,
        created_by: UserId::from(row.created_by),
        updated_by: UserId::from(row.updated_by),
        resolved_by: row.resolved_by.map(UserId::from),
        resolved_at: row.resolved_at,
        revision: Revision::new(row.revision),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

const GET_ENTRY: &str = "SELECT id, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at FROM meal_plan_entry WHERE id = $1";
const LIST_ENTRIES: &str = "SELECT id, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at FROM meal_plan_entry WHERE member_id = $1 AND planned_on >= $2 AND planned_on <= $3 ORDER BY planned_on, CASE slot WHEN 'breakfast' THEN 0 WHEN 'lunch' THEN 1 WHEN 'dinner' THEN 2 ELSE 3 END, planned_time NULLS LAST, created_at, id";
const LIST_COMPONENTS: &str = "SELECT id, entry_id, position, product_id, amount_kind, amount_value, amount_unit, frozen_product_name, nutrition_basis_amount, nutrition_basis_unit, energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra, nutrition_quality FROM meal_plan_component WHERE entry_id = ANY($1) ORDER BY entry_id, position";

pub struct PgMealPlanRepository {
    pool: PgPool,
}

impl PgMealPlanRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn components_for(&self, ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<MealPlanComponent>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<ComponentRow> = sqlx::query_as(LIST_COMPONENTS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("loading meal plan components", error))?;
        let mut grouped: HashMap<Uuid, Vec<MealPlanComponent>> = HashMap::new();
        for row in rows {
            let entry_id = row.entry_id;
            grouped
                .entry(entry_id)
                .or_default()
                .push(row.into_domain()?);
        }
        Ok(grouped)
    }
}

#[async_trait]
impl MealPlanRepository for PgMealPlanRepository {
    async fn get(&self, id: MealPlanEntryId) -> Result<Option<MealPlanEntry>> {
        let row: Option<EntryRow> = sqlx::query_as(GET_ENTRY)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| repository_error("loading a meal plan entry", error))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut components = self.components_for(&[row.id]).await?;
        let entry_id = row.id;
        Ok(Some(assemble(
            row,
            components.remove(&entry_id).unwrap_or_default(),
        )?))
    }

    async fn list(&self, query: &MealPlanQuery) -> Result<Vec<MealPlanEntry>> {
        let rows: Vec<EntryRow> = sqlx::query_as(LIST_ENTRIES)
            .bind(query.member_id.as_uuid())
            .bind(query.from)
            .bind(query.to)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing meal plan entries", error))?;
        let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
        let mut components = self.components_for(&ids).await?;
        rows.into_iter()
            .map(|row| {
                let id = row.id;
                assemble(row, components.remove(&id).unwrap_or_default())
            })
            .collect()
    }

    async fn insert(&self, entry: &MealPlanEntry) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a meal plan transaction", error))?;
        insert_entry(&mut tx, entry).await?;
        insert_components(&mut tx, entry).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a meal plan entry", error))?;
        Ok(())
    }

    async fn update(&self, entry: &MealPlanEntry, expected: Revision) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a meal plan update", error))?;
        let outcome = update_entry(&mut tx, entry, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a meal plan update", error))?;
            return Ok(outcome);
        }
        sqlx::query("DELETE FROM meal_plan_component WHERE entry_id = $1")
            .bind(entry.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| map_db_error(error, "replacing meal plan components"))?;
        insert_components(&mut tx, entry).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a meal plan update", error))?;
        Ok(UpdateOutcome::Updated)
    }

    async fn delete(&self, id: MealPlanEntryId, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM meal_plan_entry WHERE id = $1 AND revision = $2")
            .bind(id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|error| map_db_error(error, "deleting a meal plan entry"))?
            .rows_affected();
        if affected == 1 {
            Ok(UpdateOutcome::Updated)
        } else {
            current_outcome(&self.pool, id, expected).await
        }
    }

    async fn resolve(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        consumption: &[ConsumptionRecord],
    ) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a meal outcome", error))?;
        let outcome = update_entry(&mut tx, entry, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a meal outcome", error))?;
            return Ok(outcome);
        }
        freeze_components(&mut tx, entry).await?;
        for record in consumption {
            insert_consumption(&mut tx, record).await?;
        }
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a meal outcome", error))?;
        Ok(UpdateOutcome::Updated)
    }
}

async fn insert_entry(tx: &mut Transaction<'_, Postgres>, entry: &MealPlanEntry) -> Result<()> {
    sqlx::query(
        "INSERT INTO meal_plan_entry (id, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(entry.id.as_uuid())
    .bind(entry.member_id.as_uuid())
    .bind(entry.planned_on)
    .bind(entry.planned_time)
    .bind(entry.slot.code())
    .bind(entry.status.code())
    .bind(entry.created_by.as_uuid())
    .bind(entry.updated_by.as_uuid())
    .bind(entry.resolved_by.map(|id| id.as_uuid()))
    .bind(entry.resolved_at)
    .bind(entry.revision.get())
    .bind(entry.created_at)
    .bind(entry.updated_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "creating a meal plan entry"))?;
    Ok(())
}

async fn update_entry(
    tx: &mut Transaction<'_, Postgres>,
    entry: &MealPlanEntry,
    expected: Revision,
) -> Result<UpdateOutcome> {
    let affected = sqlx::query(
        "UPDATE meal_plan_entry SET member_id = $2, planned_on = $3, planned_time = $4, slot = $5, status = $6, updated_by = $7, resolved_by = $8, resolved_at = $9, revision = $10, updated_at = $11 WHERE id = $1 AND revision = $12",
    )
    .bind(entry.id.as_uuid())
    .bind(entry.member_id.as_uuid())
    .bind(entry.planned_on)
    .bind(entry.planned_time)
    .bind(entry.slot.code())
    .bind(entry.status.code())
    .bind(entry.updated_by.as_uuid())
    .bind(entry.resolved_by.map(|id| id.as_uuid()))
    .bind(entry.resolved_at)
    .bind(entry.revision.get())
    .bind(entry.updated_at)
    .bind(expected.get())
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "updating a meal plan entry"))?
    .rows_affected();
    if affected == 1 {
        return Ok(UpdateOutcome::Updated);
    }
    current_outcome(&mut **tx, entry.id, expected).await
}

async fn current_outcome<'e, E>(
    executor: E,
    id: MealPlanEntryId,
    _expected: Revision,
) -> Result<UpdateOutcome>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let current: Option<(i64,)> =
        sqlx::query_as("SELECT revision FROM meal_plan_entry WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(executor)
            .await
            .map_err(|error| repository_error("re-reading a meal plan revision", error))?;
    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}

async fn insert_components(
    tx: &mut Transaction<'_, Postgres>,
    entry: &MealPlanEntry,
) -> Result<()> {
    for component in &entry.components {
        let (kind, value, unit) = amount_bindings(&component.amount);
        sqlx::query("INSERT INTO meal_plan_component (id, entry_id, position, product_id, amount_kind, amount_value, amount_unit) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(component.id.as_uuid())
            .bind(entry.id.as_uuid())
            .bind(component.position)
            .bind(component.product_id.as_uuid())
            .bind(kind)
            .bind(value)
            .bind(unit)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a meal plan component"))?;
    }
    Ok(())
}

async fn freeze_components(
    tx: &mut Transaction<'_, Postgres>,
    entry: &MealPlanEntry,
) -> Result<()> {
    for component in &entry.components {
        let snapshot = component
            .snapshot
            .as_ref()
            .ok_or_else(|| bad_value("meal_plan_component_snapshot", "missing"))?;
        let nutrition = nutrition_bindings(&snapshot.nutrition);
        sqlx::query("UPDATE meal_plan_component SET frozen_product_name = $2, nutrition_basis_amount = $3, nutrition_basis_unit = $4, energy_kcal = $5, protein_g = $6, carbohydrate_g = $7, sugar_g = $8, fat_g = $9, saturated_fat_g = $10, fibre_g = $11, salt_g = $12, cholesterol_mg = $13, nutrition_extra = $14, nutrition_quality = $15 WHERE id = $1")
            .bind(component.id.as_uuid())
            .bind(&snapshot.product_name)
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
            .bind(snapshot.quality.code())
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "freezing a meal plan component"))?;
    }
    Ok(())
}

async fn insert_consumption(
    tx: &mut Transaction<'_, Postgres>,
    record: &ConsumptionRecord,
) -> Result<()> {
    let (kind, value, unit) = amount_bindings(&record.amount);
    let nutrition = nutrition_bindings(&record.nutrition);
    sqlx::query("INSERT INTO consumption_record (id, member_id, product_id, recorded_by, meal_plan_component_id, amount_kind, amount_value, amount_unit, consumed_on, consumed_at, nutrition_basis_amount, nutrition_basis_unit, energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra, nutrition_quality, revision, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)")
        .bind(record.id.as_uuid())
        .bind(record.member_id.as_uuid())
        .bind(record.product_id.as_uuid())
        .bind(record.recorded_by.map(|id| id.as_uuid()))
        .bind(record.meal_plan_component_id.map(|id| id.as_uuid()))
        .bind(kind)
        .bind(value)
        .bind(unit)
        .bind(record.consumed_on)
        .bind(record.consumed_at)
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
        .bind(record.quality.code())
        .bind(record.revision.get())
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, "confirming a meal plan component"))?;
    Ok(())
}
