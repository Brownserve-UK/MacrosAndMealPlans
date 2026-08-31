use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    NewStockEvent, ProductId, Revision, StockEffect, StockEffectSource, StockEvent, StockItem,
    StockItemId, StockLevel,
};
use mmp_core::ports::{Paginated, StockQuery, StockRepository, UpdateOutcome};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{StockEffectRow, StockEventRow, StockItemRow};

macro_rules! columns {
    () => {
        "id, product_id, tracking_mode, quantity_value, quantity_unit, estimated_low, \
         estimated_high, storage_location, source_date, source_date_kind, usability_deadline, \
         usability_deadline_basis, note, revision, created_at, updated_at, archived_at"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM stock_item WHERE id = $1");
const CURRENT_REVISION: &str = "SELECT revision FROM stock_item WHERE id = $1";

const LIST_ASC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM stock_item WHERE ($1 OR archived_at IS NULL) \
     AND ($2::uuid IS NULL OR product_id = $2) \
     ORDER BY created_at ASC, id ASC LIMIT $3 OFFSET $4"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM stock_item WHERE ($1 OR archived_at IS NULL) \
     AND ($2::uuid IS NULL OR product_id = $2) \
     ORDER BY created_at DESC, id DESC LIMIT $3 OFFSET $4"
);
const COUNT: &str = "SELECT count(*) FROM stock_item \
     WHERE ($1 OR archived_at IS NULL) AND ($2::uuid IS NULL OR product_id = $2)";
const LIST_FOR_PRODUCTS: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM stock_item WHERE product_id = ANY($1) ORDER BY created_at ASC, id ASC"
);

pub struct PgStockRepository {
    pool: PgPool,
}

impl PgStockRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct LevelBindings {
    tracking_mode: &'static str,
    quantity_value: Option<rust_decimal::Decimal>,
    quantity_unit: Option<&'static str>,
    estimated_low: Option<rust_decimal::Decimal>,
    estimated_high: Option<rust_decimal::Decimal>,
}

fn level_bindings(level: &StockLevel) -> LevelBindings {
    match level {
        StockLevel::Exact { quantity } => LevelBindings {
            tracking_mode: "exact",
            quantity_value: Some(quantity.amount),
            quantity_unit: Some(quantity.unit.code()),
            estimated_low: None,
            estimated_high: None,
        },
        StockLevel::Estimated { low, high, unit } => LevelBindings {
            tracking_mode: "estimated",
            quantity_value: None,
            quantity_unit: Some(unit.code()),
            estimated_low: Some(*low),
            estimated_high: Some(*high),
        },
        StockLevel::NotTracked => LevelBindings {
            tracking_mode: "not_tracked",
            quantity_value: None,
            quantity_unit: None,
            estimated_low: None,
            estimated_high: None,
        },
    }
}

async fn write_event<'e, E>(exec: E, item_id: StockItemId, event: &NewStockEvent) -> Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    let (delta, unit) = match &event.quantity_delta {
        Some(quantity) => (Some(quantity.amount), Some(quantity.unit.code())),
        None => (None, None),
    };
    sqlx::query(
        "INSERT INTO stock_event (
             id, stock_item_id, event_kind, quantity_delta, quantity_unit,
             actor_user_id, subject_member_id, source_kind, source_id, source_label,
             reverses_event_id, note
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(Uuid::now_v7())
    .bind(item_id.as_uuid())
    .bind(event.kind.code())
    .bind(delta)
    .bind(unit)
    .bind(event.actor_user_id.map(|id| id.as_uuid()))
    .bind(event.subject_member_id.map(|id| id.as_uuid()))
    .bind(event.source.as_ref().map(|s| s.kind.code()))
    .bind(event.source.as_ref().map(|s| s.id))
    .bind(event.source.as_ref().map(|s| s.label.clone()))
    .bind(event.reverses_event_id.map(|id| id.as_uuid()))
    .bind(event.note.as_deref())
    .execute(exec)
    .await
    .map_err(|e| map_db_error(e, "recording a stock event"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_stock_event(
    conn: &mut sqlx::PgConnection,
    event_id: Uuid,
    item_id: StockItemId,
    kind: mmp_core::domain::StockEventKind,
    delta: Option<mmp_core::domain::Quantity>,
    source: &mmp_core::domain::StockEventSource,
    reverses: Option<Uuid>,
    subject: Option<mmp_core::domain::HouseholdMemberId>,
    actor: Option<mmp_core::domain::UserId>,
    note: Option<&str>,
    now: time::OffsetDateTime,
) -> Result<()> {
    let (delta_value, delta_unit) = match delta {
        Some(quantity) => (Some(quantity.amount), Some(quantity.unit.code())),
        None => (None, None),
    };
    sqlx::query(
        "INSERT INTO stock_event (
             id, stock_item_id, event_kind, quantity_delta, quantity_unit,
             actor_user_id, subject_member_id, source_kind, source_id, source_label,
             reverses_event_id, note, occurred_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(event_id)
    .bind(item_id.as_uuid())
    .bind(kind.code())
    .bind(delta_value)
    .bind(delta_unit)
    .bind(actor.map(|id| id.as_uuid()))
    .bind(subject.map(|id| id.as_uuid()))
    .bind(source.kind.code())
    .bind(source.id)
    .bind(source.label.clone())
    .bind(reverses)
    .bind(note)
    .bind(now)
    .execute(&mut *conn)
    .await
    .map_err(|e| map_db_error(e, "recording a stock event"))?;
    Ok(())
}

async fn write_level(
    conn: &mut sqlx::PgConnection,
    item_id: StockItemId,
    level: &StockLevel,
    now: time::OffsetDateTime,
) -> Result<()> {
    let bindings = level_bindings(level);
    sqlx::query(
        "UPDATE stock_item SET tracking_mode = $2, quantity_value = $3, quantity_unit = $4, \
         estimated_low = $5, estimated_high = $6, revision = revision + 1, updated_at = $7 \
         WHERE id = $1",
    )
    .bind(item_id.as_uuid())
    .bind(bindings.tracking_mode)
    .bind(bindings.quantity_value)
    .bind(bindings.quantity_unit)
    .bind(bindings.estimated_low)
    .bind(bindings.estimated_high)
    .bind(now)
    .execute(&mut *conn)
    .await
    .map_err(|e| map_db_error(e, "adjusting stock level"))?;
    Ok(())
}

pub(crate) async fn apply_stock_write(
    conn: &mut sqlx::PgConnection,
    write: &mmp_core::ports::StockWrite,
    now: time::OffsetDateTime,
) -> Result<Vec<mmp_core::domain::StockOutcome>> {
    use mmp_core::domain::{
        DeductionPlan, ReleasePlan, Shortfall, StockEventKind, StockEventSource, StockOutcome,
        apply_take, plan_deduction, plan_release,
    };
    use rust_decimal::Decimal;

    let mut outcomes = Vec::new();

    for release in &write.releases {
        let effect_rows: Vec<crate::rows::StockEffectRow> = sqlx::query_as(
            "SELECT id, source_kind, source_id, stock_item_id, product_id, state, applied_mode, \
             applied_unit, exact_delta, low_delta, high_delta, requested_value, apply_event_id, \
             applied_at, released_at, note FROM stock_effect \
             WHERE source_kind = $1 AND source_id = $2 AND state = 'applied' FOR UPDATE",
        )
        .bind(release.source_kind.code())
        .bind(release.source_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| repository_error("locking stock effects for a release", e))?;
        let effects: Vec<mmp_core::domain::StockEffect> = effect_rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

        let source = StockEventSource {
            kind: release.source_kind,
            id: release.source_id,
            label: release.source_label.clone(),
        };
        let mut unresolved = false;
        let mut product_id = None;
        let mut unit = mmp_core::domain::Unit::Gram;

        for effect in effects {
            product_id = Some(effect.product_id);
            unit = effect.applied_unit;
            let item_row: Option<StockItemRow> = sqlx::query_as(concat!(
                "SELECT ",
                columns!(),
                " FROM stock_item WHERE id = $1 FOR UPDATE"
            ))
            .bind(effect.stock_item_id.as_uuid())
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| repository_error("locking a stock item for a release", e))?;
            let Some(item_row) = item_row else {
                continue;
            };
            let item: StockItem = item_row.try_into()?;

            match plan_release(&item, &effect) {
                ReleasePlan::Restored { new_level } => {
                    write_level(conn, effect.stock_item_id, &new_level, now).await?;
                    sqlx::query(
                        "UPDATE stock_effect SET state = 'released', released_at = $2 WHERE id = $1",
                    )
                    .bind(effect.id.as_uuid())
                    .bind(now)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| map_db_error(e, "releasing a stock effect"))?;
                    insert_stock_event(
                        conn,
                        Uuid::now_v7(),
                        effect.stock_item_id,
                        StockEventKind::Released,
                        None,
                        &source,
                        Some(effect.apply_event_id.as_uuid()),
                        release.subject_member_id,
                        release.actor_user_id,
                        None,
                        now,
                    )
                    .await?;
                }
                ReleasePlan::Failed { reason } => {
                    unresolved = true;
                    sqlx::query(
                        "UPDATE stock_effect SET state = 'release_failed', note = $2 WHERE id = $1",
                    )
                    .bind(effect.id.as_uuid())
                    .bind(&reason)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| map_db_error(e, "recording a failed stock release"))?;
                    insert_stock_event(
                        conn,
                        Uuid::now_v7(),
                        effect.stock_item_id,
                        StockEventKind::Released,
                        None,
                        &source,
                        Some(effect.apply_event_id.as_uuid()),
                        release.subject_member_id,
                        release.actor_user_id,
                        Some(&reason),
                        now,
                    )
                    .await?;
                }
            }
        }

        if unresolved && let Some(product_id) = product_id {
            outcomes.push(mmp_core::domain::StockOutcome {
                product_id,
                wanted: mmp_core::domain::Quantity::new(Decimal::ZERO, unit),
                deducted: mmp_core::domain::Quantity::new(Decimal::ZERO, unit),
                shortfall: Shortfall::Covered,
                unresolved_release: true,
            });
        }
    }

    for deduction in &write.deductions {
        let rows: Vec<StockItemRow> = sqlx::query_as(concat!(
            "SELECT ",
            columns!(),
            " FROM stock_item WHERE product_id = $1 AND archived_at IS NULL FOR UPDATE"
        ))
        .bind(deduction.product_id.as_uuid())
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| repository_error("locking stock for a deduction", e))?;
        let items: Vec<StockItem> = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

        let DeductionPlan::Planned { takes, shortfall } = plan_deduction(&items, deduction.want)
        else {
            continue;
        };

        let source = StockEventSource {
            kind: deduction.source_kind,
            id: deduction.source_id,
            label: deduction.source_label.clone(),
        };
        let mut deducted = Decimal::ZERO;
        for take in &takes {
            let Some(item) = items.iter().find(|i| i.id == take.stock_item_id) else {
                continue;
            };
            let Some(applied) = apply_take(&item.level, take.requested) else {
                continue;
            };
            let effect_id = Uuid::now_v7();
            let event_id = Uuid::now_v7();
            let inserted: Option<(Uuid,)> = sqlx::query_as(
                "INSERT INTO stock_effect (
                     id, source_kind, source_id, stock_item_id, product_id, state,
                     applied_mode, applied_unit, exact_delta, low_delta, high_delta,
                     requested_value, apply_event_id, applied_at
                 ) VALUES ($1, $2, $3, $4, $5, 'applied', $6, $7, $8, $9, $10, $11, $12, $13)
                 ON CONFLICT (source_kind, source_id, stock_item_id) WHERE state = 'applied'
                 DO NOTHING RETURNING id",
            )
            .bind(effect_id)
            .bind(deduction.source_kind.code())
            .bind(deduction.source_id)
            .bind(take.stock_item_id.as_uuid())
            .bind(deduction.product_id.as_uuid())
            .bind(item.tracking_mode().code())
            .bind(take.requested.unit.code())
            .bind(applied.exact_delta)
            .bind(applied.low_delta)
            .bind(applied.high_delta)
            .bind(take.requested.amount)
            .bind(event_id)
            .bind(now)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| map_db_error(e, "recording a stock effect"))?;

            if inserted.is_none() {
                continue;
            }

            let delta_amount = applied
                .exact_delta
                .or(applied.low_delta)
                .unwrap_or(Decimal::ZERO);
            insert_stock_event(
                conn,
                event_id,
                take.stock_item_id,
                StockEventKind::Consumed,
                Some(mmp_core::domain::Quantity::new(
                    delta_amount,
                    take.requested.unit,
                )),
                &source,
                None,
                deduction.subject_member_id,
                deduction.actor_user_id,
                None,
                now,
            )
            .await?;
            write_level(conn, take.stock_item_id, &applied.new_level, now).await?;

            if let Ok(converted) = take.requested.convert_to(deduction.want.unit) {
                deducted += converted.amount;
            }
        }

        if !matches!(shortfall, Shortfall::Covered) {
            outcomes.push(StockOutcome {
                product_id: deduction.product_id,
                wanted: deduction.want,
                deducted: mmp_core::domain::Quantity::new(deducted, deduction.want.unit),
                shortfall,
                unresolved_release: false,
            });
        }
    }

    Ok(outcomes)
}

#[async_trait]
impl StockRepository for PgStockRepository {
    async fn get(&self, id: StockItemId) -> Result<Option<StockItem>> {
        let row: Option<StockItemRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a stock item", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &StockQuery) -> Result<Paginated<StockItem>> {
        let sql = match query.sort {
            mmp_core::ports::SortDirection::Descending => LIST_DESC,
            mmp_core::ports::SortDirection::Ascending => LIST_ASC,
        };
        let rows: Vec<StockItemRow> = sqlx::query_as(sql)
            .bind(query.include_archived)
            .bind(query.product_id.map(|id| id.as_uuid()))
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing stock items", e))?;

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(query.product_id.map(|id| id.as_uuid()))
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting stock items", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockItem>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn list_for_products(&self, product_ids: &[ProductId]) -> Result<Vec<StockItem>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = product_ids.iter().map(|id| id.as_uuid()).collect();
        let rows: Vec<StockItemRow> = sqlx::query_as(LIST_FOR_PRODUCTS)
            .bind(&ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing stock for products", e))?;
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockItem>>>()
    }

    async fn insert(&self, item: &StockItem, event: &NewStockEvent) -> Result<()> {
        let bindings = level_bindings(&item.level);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a stock insert", e))?;

        sqlx::query(
            "INSERT INTO stock_item (
                 id, product_id, tracking_mode, quantity_value, quantity_unit,
                 estimated_low, estimated_high, storage_location,
                 source_date, source_date_kind, usability_deadline, usability_deadline_basis,
                 note, revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
        )
        .bind(item.id.as_uuid())
        .bind(item.product_id.as_uuid())
        .bind(bindings.tracking_mode)
        .bind(bindings.quantity_value)
        .bind(bindings.quantity_unit)
        .bind(bindings.estimated_low)
        .bind(bindings.estimated_high)
        .bind(item.storage_location.code())
        .bind(item.source_date.as_ref().map(|d| d.date))
        .bind(item.source_date.as_ref().map(|d| d.kind.code()))
        .bind(item.usability_deadline.as_ref().map(|d| d.date))
        .bind(
            item.usability_deadline
                .as_ref()
                .and_then(|d| d.basis.clone()),
        )
        .bind(item.note.as_deref())
        .bind(item.revision.get())
        .bind(item.created_at)
        .bind(item.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "creating a stock item"))?;

        write_event(&mut *tx, item.id, event).await?;

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a stock item", e))?;
        Ok(())
    }

    async fn update(
        &self,
        item: &StockItem,
        expected: Revision,
        event: &NewStockEvent,
    ) -> Result<UpdateOutcome> {
        let bindings = level_bindings(&item.level);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a stock update", e))?;

        let affected = sqlx::query(
            "UPDATE stock_item SET
                 tracking_mode = $2, quantity_value = $3, quantity_unit = $4,
                 estimated_low = $5, estimated_high = $6, storage_location = $7,
                 source_date = $8, source_date_kind = $9,
                 usability_deadline = $10, usability_deadline_basis = $11,
                 note = $12, archived_at = $13, revision = $14, updated_at = $15
             WHERE id = $1 AND revision = $16",
        )
        .bind(item.id.as_uuid())
        .bind(bindings.tracking_mode)
        .bind(bindings.quantity_value)
        .bind(bindings.quantity_unit)
        .bind(bindings.estimated_low)
        .bind(bindings.estimated_high)
        .bind(item.storage_location.code())
        .bind(item.source_date.as_ref().map(|d| d.date))
        .bind(item.source_date.as_ref().map(|d| d.kind.code()))
        .bind(item.usability_deadline.as_ref().map(|d| d.date))
        .bind(
            item.usability_deadline
                .as_ref()
                .and_then(|d| d.basis.clone()),
        )
        .bind(item.note.as_deref())
        .bind(item.archived_at)
        .bind(item.revision.get())
        .bind(item.updated_at)
        .bind(expected.get())
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "updating a stock item"))?
        .rows_affected();

        if affected != 1 {
            tx.rollback()
                .await
                .map_err(|e| repository_error("rolling back a stock update", e))?;
            let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
                .bind(item.id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| repository_error("re-reading a stock item revision", e))?;
            return Ok(match current {
                Some((actual,)) => UpdateOutcome::RevisionMismatch {
                    actual: Revision::new(actual),
                },
                None => UpdateOutcome::NotFound,
            });
        }

        write_event(&mut *tx, item.id, event).await?;
        tx.commit()
            .await
            .map_err(|e| repository_error("committing a stock update", e))?;
        Ok(UpdateOutcome::Updated)
    }

    async fn list_events(&self, id: StockItemId) -> Result<Vec<StockEvent>> {
        let rows: Vec<StockEventRow> = sqlx::query_as(
            "SELECT id, stock_item_id, event_kind, quantity_delta, quantity_unit, \
             actor_user_id, subject_member_id, source_kind, source_id, source_label, \
             reverses_event_id, note, occurred_at \
             FROM stock_event WHERE stock_item_id = $1 ORDER BY occurred_at DESC, id DESC",
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| repository_error("listing stock events", e))?;
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockEvent>>>()
    }

    async fn effects_for_source(
        &self,
        source_kind: StockEffectSource,
        source_id: Uuid,
    ) -> Result<Vec<StockEffect>> {
        let rows: Vec<StockEffectRow> = sqlx::query_as(
            "SELECT id, source_kind, source_id, stock_item_id, product_id, state, applied_mode, \
             applied_unit, exact_delta, low_delta, high_delta, requested_value, apply_event_id, \
             applied_at, released_at, note FROM stock_effect \
             WHERE source_kind = $1 AND source_id = $2 ORDER BY applied_at ASC, id ASC",
        )
        .bind(source_kind.code())
        .bind(source_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| repository_error("listing stock effects", e))?;
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockEffect>>>()
    }
}
