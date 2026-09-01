use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::StockOutcome;
use mmp_core::domain::{
    ConsumptionRecord, ConsumptionRecordId, MealGuestAllocation, MealGuestAllocationId,
    MealGuestGroup, MealGuestGroupId, MealItemRef, MealParticipant, MealParticipantAllocation,
    MealParticipantAllocationId, MealParticipantId, MealPlanComponent, MealPlanComponentId,
    MealPlanComponentSnapshot, MealPlanEntry, MealPlanEntryId, MealPlanScope, MealPlanStatus,
    MealSlot, NutritionFacts, NutritionQuality, ParticipantStatus, ProductId, RecipeId, Revision,
    UserId,
};
use mmp_core::ports::{
    MealPlanComponentUpdate, MealPlanQuery, MealPlanRepository, SnapshotOp, StockWrite,
    UpdateOutcome,
};

use crate::stock::apply_stock_write;
use rust_decimal::Decimal;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{
    amount_bindings, bad_value, item_bindings, nutrition_bindings, parse_amount, parse_basis,
};

type Extra = Json<BTreeMap<String, Decimal>>;

#[derive(Debug, sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    scope: String,
    member_id: Option<Uuid>,
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
    item_kind: String,
    product_id: Option<Uuid>,
    recipe_id: Option<Uuid>,
    amount_kind: String,
    amount_value: Decimal,
    amount_unit: Option<String>,
    frozen_item_name: Option<String>,
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
    status: String,
    resolved_by: Option<Uuid>,
    resolved_at: Option<OffsetDateTime>,
    revision: i64,
    display_order: Uuid,
}

impl ComponentRow {
    fn into_domain(self) -> Result<MealPlanComponent> {
        let snapshot = match (self.frozen_item_name, self.nutrition_quality) {
            (Some(item_name), Some(quality)) => Some(MealPlanComponentSnapshot {
                item_name,
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
            item: MealItemRef::from_parts(
                &self.item_kind,
                self.product_id.map(ProductId::from),
                self.recipe_id.map(RecipeId::from),
            )
            .map_err(|_| bad_value("item_kind", &self.item_kind))?,
            amount: parse_amount(&self.amount_kind, self.amount_value, self.amount_unit)?,
            position: self.position,
            snapshot,
            status: MealPlanStatus::from_str(&self.status)
                .map_err(|_| bad_value("status", &self.status))?,
            resolved_by: self.resolved_by.map(UserId::from),
            resolved_at: self.resolved_at,
            revision: Revision::new(self.revision),
            display_order: self.display_order,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ParticipantRow {
    id: Uuid,
    entry_id: Uuid,
    member_id: Uuid,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct AllocationRow {
    id: Uuid,
    participant_id: Uuid,
    component_id: Uuid,
    allocated_kind: String,
    allocated_value: Decimal,
    allocated_unit: Option<String>,
    status: String,
    consumption_record_id: Option<Uuid>,
    resolved_by: Option<Uuid>,
    resolved_at: Option<OffsetDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct GuestGroupRow {
    id: Uuid,
    entry_id: Uuid,
    guest_count: i32,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct GuestAllocationRow {
    id: Uuid,
    guest_group_id: Uuid,
    component_id: Uuid,
    allocated_kind: String,
    allocated_value: Decimal,
    allocated_unit: Option<String>,
    status: String,
    confirmed_kind: Option<String>,
    confirmed_value: Option<Decimal>,
    confirmed_unit: Option<String>,
    resolved_by: Option<Uuid>,
    resolved_at: Option<OffsetDateTime>,
}

impl GuestAllocationRow {
    fn into_domain(self) -> Result<MealGuestAllocation> {
        let confirmed = match (self.confirmed_kind, self.confirmed_value) {
            (Some(kind), Some(value)) => Some(parse_amount(&kind, value, self.confirmed_unit)?),
            (None, None) => None,
            _ => return Err(bad_value("guest confirmed amount", "partial")),
        };
        Ok(MealGuestAllocation {
            id: MealGuestAllocationId::from(self.id),
            component_id: MealPlanComponentId::from(self.component_id),
            allocated: parse_amount(
                &self.allocated_kind,
                self.allocated_value,
                self.allocated_unit,
            )?,
            status: ParticipantStatus::from_str(&self.status)
                .map_err(|_| bad_value("guest allocation status", &self.status))?,
            confirmed,
            resolved_by: self.resolved_by.map(UserId::from),
            resolved_at: self.resolved_at,
        })
    }
}

impl AllocationRow {
    fn into_domain(self) -> Result<MealParticipantAllocation> {
        Ok(MealParticipantAllocation {
            id: MealParticipantAllocationId::from(self.id),
            component_id: MealPlanComponentId::from(self.component_id),
            allocated: parse_amount(
                &self.allocated_kind,
                self.allocated_value,
                self.allocated_unit,
            )?,
            status: ParticipantStatus::from_str(&self.status)
                .map_err(|_| bad_value("allocation status", &self.status))?,
            consumption_record_id: self.consumption_record_id.map(ConsumptionRecordId::from),
            resolved_by: self.resolved_by.map(UserId::from),
            resolved_at: self.resolved_at,
        })
    }
}

fn assemble(
    row: EntryRow,
    components: Vec<MealPlanComponent>,
    participants: Vec<MealParticipant>,
    guest_groups: Vec<MealGuestGroup>,
) -> Result<MealPlanEntry> {
    let slot = MealSlot::from_str(&row.slot).map_err(|_| bad_value("slot", &row.slot))?;
    Ok(MealPlanEntry {
        id: MealPlanEntryId::from(row.id),
        scope: MealPlanScope::from_str(&row.scope).map_err(|_| bad_value("scope", &row.scope))?,
        member_id: row.member_id.map(Into::into),
        planned_on: row.planned_on,
        planned_time: row.planned_time,
        slot,
        status: MealPlanStatus::from_str(&row.status)
            .map_err(|_| bad_value("status", &row.status))?,
        components,
        participants,
        guest_groups,
        created_by: UserId::from(row.created_by),
        updated_by: UserId::from(row.updated_by),
        resolved_by: row.resolved_by.map(UserId::from),
        resolved_at: row.resolved_at,
        revision: Revision::new(row.revision),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

const GET_ENTRY: &str = "SELECT id, scope, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at FROM meal_plan_entry WHERE id = $1";
const LIST_ENTRIES: &str = "SELECT id, scope, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at FROM meal_plan_entry WHERE (member_id = $1 OR ($4 AND EXISTS (SELECT 1 FROM meal_plan_participant p WHERE p.entry_id = meal_plan_entry.id AND p.member_id = $1))) AND planned_on >= $2 AND planned_on <= $3 ORDER BY planned_on, CASE slot WHEN 'breakfast' THEN 0 WHEN 'lunch' THEN 1 WHEN 'dinner' THEN 2 ELSE 3 END, planned_time NULLS LAST, created_at, id";
const LIST_ALL_ENTRIES: &str = "SELECT id, scope, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at FROM meal_plan_entry WHERE planned_on >= $1 AND planned_on <= $2 ORDER BY planned_on, CASE slot WHEN 'breakfast' THEN 0 WHEN 'lunch' THEN 1 WHEN 'dinner' THEN 2 ELSE 3 END, planned_time NULLS LAST, created_at, id";
const LIST_COMPONENTS: &str = "SELECT id, entry_id, position, item_kind, product_id, recipe_id, amount_kind, amount_value, amount_unit, frozen_item_name, nutrition_basis_amount, nutrition_basis_unit, energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra, nutrition_quality, status, resolved_by, resolved_at, revision, display_order FROM meal_plan_component WHERE entry_id = ANY($1) ORDER BY entry_id, position";
const LIST_PARTICIPANTS: &str = "SELECT id, entry_id, member_id, revision, created_at, updated_at FROM meal_plan_participant WHERE entry_id = ANY($1) ORDER BY entry_id, created_at, id";
const LIST_ALLOCATIONS: &str = "SELECT id, participant_id, component_id, allocated_kind, allocated_value, allocated_unit, status, consumption_record_id, resolved_by, resolved_at FROM meal_plan_participant_allocation WHERE participant_id = ANY($1)";
const LIST_GUEST_GROUPS: &str = "SELECT id, entry_id, guest_count, revision, created_at, updated_at FROM meal_guest_group WHERE entry_id = ANY($1) ORDER BY entry_id, created_at, id";
const LIST_GUEST_ALLOCATIONS: &str = "SELECT id, guest_group_id, component_id, allocated_kind, allocated_value, allocated_unit, status, confirmed_kind, confirmed_value, confirmed_unit, resolved_by, resolved_at FROM meal_guest_allocation WHERE guest_group_id = ANY($1)";

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

    async fn participants_for(&self, ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<MealParticipant>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let participant_rows: Vec<ParticipantRow> = sqlx::query_as(LIST_PARTICIPANTS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("loading meal plan participants", error))?;
        let participant_ids: Vec<Uuid> = participant_rows.iter().map(|row| row.id).collect();
        let allocation_rows: Vec<AllocationRow> = if participant_ids.is_empty() {
            Vec::new()
        } else {
            sqlx::query_as(LIST_ALLOCATIONS)
                .bind(&participant_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| repository_error("loading participant allocations", error))?
        };
        let mut allocations_by_participant: HashMap<Uuid, Vec<MealParticipantAllocation>> =
            HashMap::new();
        for row in allocation_rows {
            let participant_id = row.participant_id;
            allocations_by_participant
                .entry(participant_id)
                .or_default()
                .push(row.into_domain()?);
        }
        let mut grouped: HashMap<Uuid, Vec<MealParticipant>> = HashMap::new();
        for row in participant_rows {
            let allocations = allocations_by_participant
                .remove(&row.id)
                .unwrap_or_default();
            grouped
                .entry(row.entry_id)
                .or_default()
                .push(MealParticipant {
                    id: MealParticipantId::from(row.id),
                    member_id: row.member_id.into(),
                    allocations,
                    revision: Revision::new(row.revision),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                });
        }
        Ok(grouped)
    }

    async fn guests_for(&self, ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<MealGuestGroup>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let group_rows: Vec<GuestGroupRow> = sqlx::query_as(LIST_GUEST_GROUPS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("loading meal guests", error))?;
        let group_ids: Vec<Uuid> = group_rows.iter().map(|row| row.id).collect();
        let allocation_rows: Vec<GuestAllocationRow> = if group_ids.is_empty() {
            Vec::new()
        } else {
            sqlx::query_as(LIST_GUEST_ALLOCATIONS)
                .bind(&group_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| repository_error("loading guest portions", error))?
        };
        let mut allocations_by_group: HashMap<Uuid, Vec<MealGuestAllocation>> = HashMap::new();
        for row in allocation_rows {
            let group_id = row.guest_group_id;
            allocations_by_group
                .entry(group_id)
                .or_default()
                .push(row.into_domain()?);
        }
        let mut grouped: HashMap<Uuid, Vec<MealGuestGroup>> = HashMap::new();
        for row in group_rows {
            grouped
                .entry(row.entry_id)
                .or_default()
                .push(MealGuestGroup {
                    id: MealGuestGroupId::from(row.id),
                    count: row.guest_count,
                    allocations: allocations_by_group.remove(&row.id).unwrap_or_default(),
                    revision: Revision::new(row.revision),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                });
        }
        Ok(grouped)
    }

    async fn hydrate(&self, rows: Vec<EntryRow>) -> Result<Vec<MealPlanEntry>> {
        let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let mut components = self.components_for(&ids).await?;
        let mut participants = self.participants_for(&ids).await?;
        let mut guests = self.guests_for(&ids).await?;
        rows.into_iter()
            .map(|row| {
                let id = row.id;
                assemble(
                    row,
                    components.remove(&id).unwrap_or_default(),
                    participants.remove(&id).unwrap_or_default(),
                    guests.remove(&id).unwrap_or_default(),
                )
            })
            .collect()
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
        Ok(self.hydrate(vec![row]).await?.into_iter().next())
    }

    async fn list(&self, query: &MealPlanQuery) -> Result<Vec<MealPlanEntry>> {
        let rows: Vec<EntryRow> = sqlx::query_as(LIST_ENTRIES)
            .bind(query.member_id.as_uuid())
            .bind(query.from)
            .bind(query.to)
            .bind(query.include_participating)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing meal plan entries", error))?;
        self.hydrate(rows).await
    }

    async fn list_all(&self, from: Date, to: Date) -> Result<Vec<MealPlanEntry>> {
        let rows: Vec<EntryRow> = sqlx::query_as(LIST_ALL_ENTRIES)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing Planner meals", error))?;
        self.hydrate(rows).await
    }

    async fn insert(&self, entry: &MealPlanEntry) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a meal plan transaction", error))?;
        insert_entry(&mut tx, entry).await?;
        insert_components(&mut tx, entry).await?;
        insert_participants(&mut tx, entry).await?;
        insert_guests(&mut tx, entry.id, &entry.guest_groups).await?;
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
        sqlx::query("DELETE FROM meal_plan_participant WHERE entry_id = $1")
            .bind(entry.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| map_db_error(error, "clearing meal plan participants"))?;
        sqlx::query("DELETE FROM meal_guest_group WHERE entry_id = $1")
            .bind(entry.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| map_db_error(error, "clearing meal guests"))?;
        sqlx::query("DELETE FROM meal_plan_component WHERE entry_id = $1")
            .bind(entry.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| map_db_error(error, "replacing meal plan components"))?;
        insert_components(&mut tx, entry).await?;
        insert_participants(&mut tx, entry).await?;
        insert_guests(&mut tx, entry.id, &entry.guest_groups).await?;
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
            current_outcome(&self.pool, id).await
        }
    }

    async fn resolve(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        consumption: &[ConsumptionRecord],
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
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
            return Ok((outcome, Vec::new()));
        }
        persist_components(&mut tx, entry).await?;
        for record in consumption {
            insert_consumption(&mut tx, record).await?;
        }
        replace_participants(&mut tx, entry.id, &entry.participants).await?;
        replace_guests(&mut tx, entry.id, &entry.guest_groups).await?;
        let stock_outcomes = apply_stock_write(&mut tx, stock, OffsetDateTime::now_utc()).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a meal outcome", error))?;
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }

    async fn reopen(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        delete_records: &[ConsumptionRecordId],
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a meal reopen", error))?;
        let outcome = update_entry(&mut tx, entry, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a meal reopen", error))?;
            return Ok((outcome, Vec::new()));
        }
        persist_components(&mut tx, entry).await?;
        replace_participants(&mut tx, entry.id, &entry.participants).await?;
        replace_guests(&mut tx, entry.id, &entry.guest_groups).await?;
        for record_id in delete_records {
            delete_consumption(&mut tx, *record_id).await?;
        }
        let stock_outcomes = apply_stock_write(&mut tx, stock, OffsetDateTime::now_utc()).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a meal reopen", error))?;
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }

    async fn set_participants(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a participant update", error))?;
        let outcome = update_entry(&mut tx, entry, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a participant update", error))?;
            return Ok(outcome);
        }
        replace_participants(&mut tx, entry.id, &entry.participants).await?;
        replace_guests(&mut tx, entry.id, &entry.guest_groups).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a participant update", error))?;
        Ok(UpdateOutcome::Updated)
    }

    async fn resolve_component(
        &self,
        entry_id: MealPlanEntryId,
        component: &MealPlanComponentUpdate<'_>,
        participants: &[MealParticipant],
        expected: Revision,
        consumption: Option<&ConsumptionRecord>,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a component outcome", error))?;
        let outcome = update_component(&mut tx, entry_id, component, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a component outcome", error))?;
            return Ok((outcome, Vec::new()));
        }
        if let Some(record) = consumption {
            insert_consumption(&mut tx, record).await?;
        }
        replace_participants(&mut tx, entry_id, participants).await?;
        update_entry_state(&mut tx, entry_id, component).await?;
        let stock_outcomes = apply_stock_write(&mut tx, stock, OffsetDateTime::now_utc()).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a component outcome", error))?;
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }

    async fn reopen_component(
        &self,
        entry_id: MealPlanEntryId,
        component: &MealPlanComponentUpdate<'_>,
        participants: &[MealParticipant],
        expected: Revision,
        delete_record: Option<ConsumptionRecordId>,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a component reopen", error))?;
        let outcome = update_component(&mut tx, entry_id, component, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a component reopen", error))?;
            return Ok((outcome, Vec::new()));
        }
        replace_participants(&mut tx, entry_id, participants).await?;
        if let Some(record_id) = delete_record {
            delete_consumption(&mut tx, record_id).await?;
        }
        update_entry_state(&mut tx, entry_id, component).await?;
        let stock_outcomes = apply_stock_write(&mut tx, stock, OffsetDateTime::now_utc()).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a component reopen", error))?;
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }
}

async fn delete_consumption(
    tx: &mut Transaction<'_, Postgres>,
    id: ConsumptionRecordId,
) -> Result<()> {
    sqlx::query("DELETE FROM consumption_record WHERE id = $1")
        .bind(id.as_uuid())
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, "removing a consumption record"))?;
    Ok(())
}

async fn insert_entry(tx: &mut Transaction<'_, Postgres>, entry: &MealPlanEntry) -> Result<()> {
    sqlx::query(
        "INSERT INTO meal_plan_entry (id, scope, member_id, planned_on, planned_time, slot, status, created_by, updated_by, resolved_by, resolved_at, revision, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(entry.id.as_uuid())
    .bind(entry.scope.code())
    .bind(entry.member_id.map(|id| id.as_uuid()))
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
        "UPDATE meal_plan_entry SET planned_on = $2, planned_time = $3, slot = $4, status = $5, updated_by = $6, resolved_by = $7, resolved_at = $8, revision = $9, updated_at = $10 WHERE id = $1 AND revision = $11",
    )
    .bind(entry.id.as_uuid())
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
    current_outcome(&mut **tx, entry.id).await
}

async fn current_outcome<'e, E>(executor: E, id: MealPlanEntryId) -> Result<UpdateOutcome>
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

async fn current_component_outcome<'e, E>(
    executor: E,
    id: MealPlanComponentId,
) -> Result<UpdateOutcome>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let current: Option<(i64,)> =
        sqlx::query_as("SELECT revision FROM meal_plan_component WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(executor)
            .await
            .map_err(|error| repository_error("re-reading a component revision", error))?;
    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}

async fn update_component(
    tx: &mut Transaction<'_, Postgres>,
    entry_id: MealPlanEntryId,
    update: &MealPlanComponentUpdate<'_>,
    expected: Revision,
) -> Result<UpdateOutcome> {
    let affected = match update.snapshot {
        SnapshotOp::Keep => {
            sqlx::query(
                "UPDATE meal_plan_component SET status = $3, resolved_by = $4, resolved_at = $5, revision = $6 WHERE id = $1 AND entry_id = $2 AND revision = $7",
            )
            .bind(update.id.as_uuid())
            .bind(entry_id.as_uuid())
            .bind(update.status.code())
            .bind(update.resolved_by.map(|id| id.as_uuid()))
            .bind(update.resolved_at)
            .bind(update.revision.get())
            .bind(expected.get())
            .execute(&mut **tx)
            .await
        }
        SnapshotOp::Clear => {
            sqlx::query(
                "UPDATE meal_plan_component SET status = $3, resolved_by = $4, resolved_at = $5, revision = $6, frozen_item_name = NULL, nutrition_basis_amount = NULL, nutrition_basis_unit = NULL, energy_kcal = NULL, protein_g = NULL, carbohydrate_g = NULL, sugar_g = NULL, fat_g = NULL, saturated_fat_g = NULL, fibre_g = NULL, salt_g = NULL, cholesterol_mg = NULL, nutrition_extra = NULL, nutrition_quality = NULL WHERE id = $1 AND entry_id = $2 AND revision = $7",
            )
            .bind(update.id.as_uuid())
            .bind(entry_id.as_uuid())
            .bind(update.status.code())
            .bind(update.resolved_by.map(|id| id.as_uuid()))
            .bind(update.resolved_at)
            .bind(update.revision.get())
            .bind(expected.get())
            .execute(&mut **tx)
            .await
        }
        SnapshotOp::Set(snapshot) => {
            let nutrition = nutrition_bindings(&snapshot.nutrition);
            sqlx::query(
                "UPDATE meal_plan_component SET status = $3, resolved_by = $4, resolved_at = $5, revision = $6, frozen_item_name = COALESCE(frozen_item_name, $7), nutrition_basis_amount = COALESCE(nutrition_basis_amount, $8), nutrition_basis_unit = COALESCE(nutrition_basis_unit, $9), energy_kcal = COALESCE(energy_kcal, $10), protein_g = COALESCE(protein_g, $11), carbohydrate_g = COALESCE(carbohydrate_g, $12), sugar_g = COALESCE(sugar_g, $13), fat_g = COALESCE(fat_g, $14), saturated_fat_g = COALESCE(saturated_fat_g, $15), fibre_g = COALESCE(fibre_g, $16), salt_g = COALESCE(salt_g, $17), cholesterol_mg = COALESCE(cholesterol_mg, $18), nutrition_extra = COALESCE(nutrition_extra, $19), nutrition_quality = COALESCE(nutrition_quality, $20) WHERE id = $1 AND entry_id = $2 AND revision = $21",
            )
            .bind(update.id.as_uuid())
            .bind(entry_id.as_uuid())
            .bind(update.status.code())
            .bind(update.resolved_by.map(|id| id.as_uuid()))
            .bind(update.resolved_at)
            .bind(update.revision.get())
            .bind(&snapshot.item_name)
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
            .bind(expected.get())
            .execute(&mut **tx)
            .await
        }
    }
    .map_err(|error| map_db_error(error, "resolving a meal plan component"))?
    .rows_affected();
    if affected == 1 {
        Ok(UpdateOutcome::Updated)
    } else {
        current_component_outcome(&mut **tx, update.id).await
    }
}

async fn update_entry_state(
    tx: &mut Transaction<'_, Postgres>,
    entry_id: MealPlanEntryId,
    update: &MealPlanComponentUpdate<'_>,
) -> Result<()> {
    sqlx::query(
        "UPDATE meal_plan_entry SET status = $2, resolved_by = $3, resolved_at = $4, updated_by = $5, updated_at = $6, revision = revision + 1 WHERE id = $1",
    )
    .bind(entry_id.as_uuid())
    .bind(update.entry_status.code())
    .bind(update.entry_resolved_by.map(|id| id.as_uuid()))
    .bind(update.entry_resolved_at)
    .bind(update.actor_id.as_uuid())
    .bind(update.now)
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "refreshing meal progress"))?;
    Ok(())
}

async fn insert_components(
    tx: &mut Transaction<'_, Postgres>,
    entry: &MealPlanEntry,
) -> Result<()> {
    for component in &entry.components {
        let (kind, value, unit) = amount_bindings(&component.amount);
        let (item_kind, item_product_id, item_recipe_id) = item_bindings(&component.item);
        let mut query = sqlx::query("INSERT INTO meal_plan_component (id, entry_id, position, product_id, amount_kind, amount_value, amount_unit, status, resolved_by, resolved_at, revision, display_order, item_kind, recipe_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)")
            .bind(component.id.as_uuid())
            .bind(entry.id.as_uuid())
            .bind(component.position)
            .bind(item_product_id)
            .bind(kind)
            .bind(value)
            .bind(unit)
            .bind(component.status.code())
            .bind(component.resolved_by.map(|id| id.as_uuid()))
            .bind(component.resolved_at)
            .bind(component.revision.get())
            .bind(component.display_order)
            .bind(item_kind)
            .bind(item_recipe_id);
        let _ = &mut query;
        query
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a meal plan component"))?;
        if let Some(snapshot) = &component.snapshot {
            write_component_snapshot(tx, component.id, snapshot).await?;
        }
    }
    Ok(())
}

async fn persist_components(
    tx: &mut Transaction<'_, Postgres>,
    entry: &MealPlanEntry,
) -> Result<()> {
    for component in &entry.components {
        sqlx::query("UPDATE meal_plan_component SET status = $2, resolved_by = $3, resolved_at = $4, revision = $5 WHERE id = $1")
            .bind(component.id.as_uuid())
            .bind(component.status.code())
            .bind(component.resolved_by.map(|id| id.as_uuid()))
            .bind(component.resolved_at)
            .bind(component.revision.get())
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "updating a meal plan component"))?;
        match &component.snapshot {
            Some(snapshot) => write_component_snapshot(tx, component.id, snapshot).await?,
            None => {
                sqlx::query("UPDATE meal_plan_component SET frozen_item_name = NULL, nutrition_basis_amount = NULL, nutrition_basis_unit = NULL, energy_kcal = NULL, protein_g = NULL, carbohydrate_g = NULL, sugar_g = NULL, fat_g = NULL, saturated_fat_g = NULL, fibre_g = NULL, salt_g = NULL, cholesterol_mg = NULL, nutrition_extra = NULL, nutrition_quality = NULL WHERE id = $1")
                    .bind(component.id.as_uuid())
                    .execute(&mut **tx)
                    .await
                    .map_err(|error| map_db_error(error, "clearing a meal plan component snapshot"))?;
            }
        }
    }
    Ok(())
}

async fn write_component_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    component_id: MealPlanComponentId,
    snapshot: &MealPlanComponentSnapshot,
) -> Result<()> {
    let nutrition = nutrition_bindings(&snapshot.nutrition);
    sqlx::query("UPDATE meal_plan_component SET frozen_item_name = $2, nutrition_basis_amount = $3, nutrition_basis_unit = $4, energy_kcal = $5, protein_g = $6, carbohydrate_g = $7, sugar_g = $8, fat_g = $9, saturated_fat_g = $10, fibre_g = $11, salt_g = $12, cholesterol_mg = $13, nutrition_extra = $14, nutrition_quality = $15 WHERE id = $1")
        .bind(component_id.as_uuid())
        .bind(&snapshot.item_name)
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
    Ok(())
}

async fn insert_participants(
    tx: &mut Transaction<'_, Postgres>,
    entry: &MealPlanEntry,
) -> Result<()> {
    for participant in &entry.participants {
        sqlx::query("INSERT INTO meal_plan_participant (id, entry_id, member_id, planned_on, planned_time, slot, revision, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(participant.id.as_uuid())
            .bind(entry.id.as_uuid())
            .bind(participant.member_id.as_uuid())
            .bind(entry.planned_on)
            .bind(entry.planned_time)
            .bind(entry.slot.code())
            .bind(participant.revision.get())
            .bind(participant.created_at)
            .bind(participant.updated_at)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a meal plan participant"))?;
        for allocation in &participant.allocations {
            let (kind, value, unit) = amount_bindings(&allocation.allocated);
            sqlx::query("INSERT INTO meal_plan_participant_allocation (id, entry_id, participant_id, component_id, allocated_kind, allocated_value, allocated_unit, status, consumption_record_id, resolved_by, resolved_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
                .bind(allocation.id.as_uuid())
                .bind(entry.id.as_uuid())
                .bind(participant.id.as_uuid())
                .bind(allocation.component_id.as_uuid())
                .bind(kind)
                .bind(value)
                .bind(unit)
                .bind(allocation.status.code())
                .bind(allocation.consumption_record_id.map(|id| id.as_uuid()))
                .bind(allocation.resolved_by.map(|id| id.as_uuid()))
                .bind(allocation.resolved_at)
                .execute(&mut **tx)
                .await
                .map_err(|error| map_db_error(error, "creating a participant allocation"))?;
        }
    }
    Ok(())
}

async fn replace_participants(
    tx: &mut Transaction<'_, Postgres>,
    entry_id: MealPlanEntryId,
    participants: &[MealParticipant],
) -> Result<()> {
    let row: (Date, Option<Time>, String) =
        sqlx::query_as("SELECT planned_on, planned_time, slot FROM meal_plan_entry WHERE id = $1")
            .bind(entry_id.as_uuid())
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| repository_error("reading a meal plan occurrence", error))?;
    sqlx::query("DELETE FROM meal_plan_participant WHERE entry_id = $1")
        .bind(entry_id.as_uuid())
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, "clearing meal plan participants"))?;
    for participant in participants {
        sqlx::query("INSERT INTO meal_plan_participant (id, entry_id, member_id, planned_on, planned_time, slot, revision, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(participant.id.as_uuid())
            .bind(entry_id.as_uuid())
            .bind(participant.member_id.as_uuid())
            .bind(row.0)
            .bind(row.1)
            .bind(&row.2)
            .bind(participant.revision.get())
            .bind(participant.created_at)
            .bind(participant.updated_at)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a meal plan participant"))?;
        for allocation in &participant.allocations {
            let (kind, value, unit) = amount_bindings(&allocation.allocated);
            sqlx::query("INSERT INTO meal_plan_participant_allocation (id, entry_id, participant_id, component_id, allocated_kind, allocated_value, allocated_unit, status, consumption_record_id, resolved_by, resolved_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
                .bind(allocation.id.as_uuid())
                .bind(entry_id.as_uuid())
                .bind(participant.id.as_uuid())
                .bind(allocation.component_id.as_uuid())
                .bind(kind)
                .bind(value)
                .bind(unit)
                .bind(allocation.status.code())
                .bind(allocation.consumption_record_id.map(|id| id.as_uuid()))
                .bind(allocation.resolved_by.map(|id| id.as_uuid()))
                .bind(allocation.resolved_at)
                .execute(&mut **tx)
                .await
                .map_err(|error| map_db_error(error, "creating a participant allocation"))?;
        }
    }
    Ok(())
}

async fn insert_guests(
    tx: &mut Transaction<'_, Postgres>,
    entry_id: MealPlanEntryId,
    groups: &[MealGuestGroup],
) -> Result<()> {
    for group in groups {
        sqlx::query("INSERT INTO meal_guest_group (id, entry_id, guest_count, revision, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(group.id.as_uuid())
            .bind(entry_id.as_uuid())
            .bind(group.count)
            .bind(group.revision.get())
            .bind(group.created_at)
            .bind(group.updated_at)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating meal guests"))?;
        for allocation in &group.allocations {
            let (kind, value, unit) = amount_bindings(&allocation.allocated);
            let (confirmed_kind, confirmed_value, confirmed_unit) = allocation
                .confirmed
                .as_ref()
                .map(amount_bindings)
                .map_or((None, None, None), |(kind, value, unit)| {
                    (Some(kind), Some(value), unit)
                });
            sqlx::query("INSERT INTO meal_guest_allocation (id, entry_id, guest_group_id, component_id, allocated_kind, allocated_value, allocated_unit, status, confirmed_kind, confirmed_value, confirmed_unit, resolved_by, resolved_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)")
                .bind(allocation.id.as_uuid())
                .bind(entry_id.as_uuid())
                .bind(group.id.as_uuid())
                .bind(allocation.component_id.as_uuid())
                .bind(kind)
                .bind(value)
                .bind(unit)
                .bind(allocation.status.code())
                .bind(confirmed_kind)
                .bind(confirmed_value)
                .bind(confirmed_unit)
                .bind(allocation.resolved_by.map(|id| id.as_uuid()))
                .bind(allocation.resolved_at)
                .execute(&mut **tx)
                .await
                .map_err(|error| map_db_error(error, "creating guest portions"))?;
        }
    }
    Ok(())
}

async fn replace_guests(
    tx: &mut Transaction<'_, Postgres>,
    entry_id: MealPlanEntryId,
    groups: &[MealGuestGroup],
) -> Result<()> {
    sqlx::query("DELETE FROM meal_guest_group WHERE entry_id = $1")
        .bind(entry_id.as_uuid())
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, "clearing meal guests"))?;
    insert_guests(tx, entry_id, groups).await
}

async fn insert_consumption(
    tx: &mut Transaction<'_, Postgres>,
    record: &ConsumptionRecord,
) -> Result<()> {
    let (kind, value, unit) = amount_bindings(&record.amount);
    let (item_kind, item_product_id, item_recipe_id) = item_bindings(&record.item);
    let nutrition = nutrition_bindings(&record.nutrition);
    sqlx::query("INSERT INTO consumption_record (id, member_id, product_id, recorded_by, meal_plan_component_id, slot, amount_kind, amount_value, amount_unit, consumed_on, consumed_at, nutrition_basis_amount, nutrition_basis_unit, energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra, nutrition_quality, revision, created_at, updated_at, item_kind, recipe_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29)")
        .bind(record.id.as_uuid())
        .bind(record.member_id.as_uuid())
        .bind(item_product_id)
        .bind(record.recorded_by.map(|id| id.as_uuid()))
        .bind(record.meal_plan_component_id.map(|id| id.as_uuid()))
        .bind(record.slot.code())
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
        .bind(item_kind)
        .bind(item_recipe_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, "confirming a meal plan component"))?;
    Ok(())
}
