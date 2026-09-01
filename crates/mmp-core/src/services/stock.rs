use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;

use crate::domain::{
    Availability, Confidence, HouseholdMemberId, MissingStockInterpretation, NewStockEvent,
    NewStockItem, ProductAvailability, ProductId, Quantity, Revision, StockEvent, StockEventKind,
    StockItem, StockItemId, StockItemPatch, UserId,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, HouseholdMemberRepository, HouseholdSettingsRepository, MealPlanQuery,
    MealPlanRepository, MemberQuery, PageRequest, Paginated, ProductRepository, StockQuery,
    StockRepository, UpdateOutcome,
};

const STOCK_ITEM: &str = "stock item";

#[derive(Clone)]
pub struct StockService {
    stock: Arc<dyn StockRepository>,
    products: Arc<dyn ProductRepository>,
    meal_plans: Arc<dyn MealPlanRepository>,
    members: Arc<dyn HouseholdMemberRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl StockService {
    pub fn new(
        stock: Arc<dyn StockRepository>,
        products: Arc<dyn ProductRepository>,
        meal_plans: Arc<dyn MealPlanRepository>,
        members: Arc<dyn HouseholdMemberRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            stock,
            products,
            meal_plans,
            members,
            settings,
            clock,
        }
    }

    pub async fn list(&self, query: &StockQuery) -> Result<Paginated<StockItem>> {
        self.stock.list(query).await
    }

    pub async fn get(&self, id: StockItemId) -> Result<StockItem> {
        self.stock
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(STOCK_ITEM, id))
    }

    pub async fn events(&self, id: StockItemId) -> Result<Vec<StockEvent>> {
        self.get(id).await?;
        self.stock.list_events(id).await
    }

    pub async fn effects_for_source(
        &self,
        source_kind: crate::domain::StockEffectSource,
        source_id: uuid::Uuid,
    ) -> Result<Vec<crate::domain::StockEffect>> {
        self.stock.effects_for_source(source_kind, source_id).await
    }

    pub async fn create(
        &self,
        input: NewStockItem,
        actor: UserId,
        subject: Option<HouseholdMemberId>,
    ) -> Result<StockItem> {
        input.validate()?;
        let product = self
            .products
            .get(input.product_id)
            .await?
            .ok_or_else(|| CoreError::not_found("product", input.product_id))?;
        if product.is_archived() {
            return Err(CoreError::conflict("That product is archived."));
        }

        let now = self.clock.now();
        let item = StockItem {
            id: StockItemId::new(),
            product_id: input.product_id,
            level: input.level,
            storage_location: input.storage_location,
            source_date: input.source_date,
            usability_deadline: input.usability_deadline,
            note: normalise(input.note),
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        let event = NewStockEvent {
            kind: StockEventKind::Added,
            quantity_delta: item.level.conservative_quantity(),
            actor_user_id: Some(actor),
            subject_member_id: subject,
            source: None,
            reverses_event_id: None,
            note: None,
        };
        self.stock.insert(&item, &event).await?;
        Ok(item)
    }

    pub async fn update(
        &self,
        id: StockItemId,
        expected: Revision,
        patch: StockItemPatch,
        actor: UserId,
        subject: Option<HouseholdMemberId>,
    ) -> Result<StockItem> {
        patch.validate()?;
        let mut current = self.get(id).await?;
        require_revision(id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        let previous_mode = current.tracking_mode();
        if let Some(level) = patch.level {
            current.level = level;
        }
        if let Some(location) = patch.storage_location {
            current.storage_location = location;
        }
        current.source_date = patch.source_date.apply(current.source_date);
        current.usability_deadline = patch.usability_deadline.apply(current.usability_deadline);
        current.note = normalise(patch.note.apply(current.note));

        let kind = if current.tracking_mode() != previous_mode {
            StockEventKind::ModeChanged
        } else {
            StockEventKind::Corrected
        };
        current.revision = current.revision.next();
        current.updated_at = self.clock.now();

        let event = NewStockEvent {
            kind,
            quantity_delta: current.level.conservative_quantity(),
            actor_user_id: Some(actor),
            subject_member_id: subject,
            source: None,
            reverses_event_id: None,
            note: None,
        };
        commit(
            self.stock.update(&current, expected, &event).await?,
            id,
            expected,
        )?;
        Ok(current)
    }

    pub async fn set_archived(
        &self,
        id: StockItemId,
        expected: Revision,
        actor: UserId,
    ) -> Result<StockItem> {
        let mut current = self.get(id).await?;
        require_revision(id, expected, current.revision)?;
        if current.archived_at.is_none() {
            current.archived_at = Some(self.clock.now());
        }
        current.revision = current.revision.next();
        current.updated_at = self.clock.now();
        let event = NewStockEvent {
            kind: StockEventKind::Archived,
            quantity_delta: None,
            actor_user_id: Some(actor),
            subject_member_id: None,
            source: None,
            reverses_event_id: None,
            note: None,
        };
        commit(
            self.stock.update(&current, expected, &event).await?,
            id,
            expected,
        )?;
        Ok(current)
    }

    pub async fn availability_overview(
        &self,
        from: Date,
        to: Date,
    ) -> Result<Vec<ProductAvailability>> {
        let all = self
            .stock
            .list(&StockQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        let mut ids: Vec<ProductId> = all.items.iter().map(|item| item.product_id).collect();
        ids.sort_unstable_by_key(|id| id.as_uuid());
        ids.dedup();
        self.availability(&ids, from, to).await
    }

    pub async fn availability(
        &self,
        product_ids: &[ProductId],
        from: Date,
        to: Date,
    ) -> Result<Vec<ProductAvailability>> {
        let interpretation = self.settings.get().await?.missing_stock_interpretation;
        let items = self.stock.list_for_products(product_ids).await?;

        let mut by_product: HashMap<ProductId, Vec<&StockItem>> = HashMap::new();
        for item in &items {
            if item.is_archived() {
                continue;
            }
            by_product.entry(item.product_id).or_default().push(item);
        }

        let (demand, demand_incomplete) = self.planned_demand(from, to).await?;

        let mut out = Vec::with_capacity(product_ids.len());
        for &product_id in product_ids {
            let stock = by_product
                .get(&product_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let want = demand.get(&product_id).copied();
            out.push(ProductAvailability {
                product_id,
                availability: resolve_availability(stock, want, interpretation),
                demand_incomplete,
            });
        }
        Ok(out)
    }

    async fn planned_demand(
        &self,
        from: Date,
        to: Date,
    ) -> Result<(HashMap<ProductId, Quantity>, bool)> {
        let members = self
            .members
            .list(&MemberQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;

        let mut demand: HashMap<ProductId, Quantity> = HashMap::new();
        let mut incomplete = false;
        let mut product_cache: HashMap<ProductId, Option<crate::domain::Product>> = HashMap::new();

        let mut entries: Vec<crate::domain::MealPlanEntry> = Vec::new();
        for member in members.items {
            entries.extend(
                self.meal_plans
                    .list(&MealPlanQuery {
                        member_id: member.id,
                        from,
                        to,
                        include_participating: false,
                    })
                    .await?,
            );
        }
        entries.extend(
            self.meal_plans
                .list_all(from, to)
                .await?
                .into_iter()
                .filter(|entry| entry.scope == crate::domain::MealPlanScope::Household),
        );

        for entry in entries {
            let household = entry.scope == crate::domain::MealPlanScope::Household;
            for component in &entry.components {
                if !component_is_future_demand(&entry, component.id) {
                    continue;
                }
                let Some(product_id) = component.item.product_id() else {
                    incomplete = true;
                    continue;
                };
                let product = match product_cache.get(&product_id) {
                    Some(cached) => cached.clone(),
                    None => {
                        let loaded = self.products.get(product_id).await?;
                        product_cache.insert(product_id, loaded.clone());
                        loaded
                    }
                };
                let Some(product) = product else {
                    incomplete = true;
                    continue;
                };
                let wanted = if household {
                    demand_amount_for_household(&entry, component)
                } else {
                    component.amount
                };
                match wanted.resolve(&product) {
                    Ok(quantity) => add_demand(&mut demand, product_id, quantity, &mut incomplete),
                    Err(_) => incomplete = true,
                }
            }
        }
        Ok((demand, incomplete))
    }
}

fn demand_amount_for_household(
    entry: &crate::domain::MealPlanEntry,
    component: &crate::domain::MealPlanComponent,
) -> crate::domain::ConsumedAmount {
    let mut allocations: Vec<crate::domain::ConsumedAmount> = entry
        .participants
        .iter()
        .flat_map(|participant| participant.allocations.iter())
        .filter(|allocation| allocation.component_id == component.id)
        .map(|allocation| allocation.allocated)
        .collect();
    for group in &entry.guest_groups {
        for allocation in group
            .allocations
            .iter()
            .filter(|allocation| allocation.component_id == component.id)
        {
            for _ in 0..group.count.max(0) {
                allocations.push(allocation.allocated);
            }
        }
    }
    if allocations.is_empty() {
        return component.amount;
    }
    crate::domain::allocated_total(&component.amount, &allocations).unwrap_or(component.amount)
}

fn component_is_future_demand(
    entry: &crate::domain::MealPlanEntry,
    component_id: crate::domain::MealPlanComponentId,
) -> bool {
    let statuses: Vec<crate::domain::ParticipantStatus> = entry
        .participants
        .iter()
        .flat_map(|participant| participant.allocations.iter())
        .filter(|allocation| allocation.component_id == component_id)
        .map(|allocation| allocation.status)
        .collect();
    if statuses.is_empty() {
        return true;
    }
    if statuses.contains(&crate::domain::ParticipantStatus::Eaten) {
        return false;
    }
    statuses.contains(&crate::domain::ParticipantStatus::Planned)
}

fn add_demand(
    demand: &mut HashMap<ProductId, Quantity>,
    product_id: ProductId,
    quantity: Quantity,
    incomplete: &mut bool,
) {
    match demand.get(&product_id).copied() {
        None => {
            demand.insert(product_id, quantity);
        }
        Some(existing) => match quantity.convert_to(existing.unit) {
            Ok(converted) => {
                demand.insert(
                    product_id,
                    Quantity::new(existing.amount + converted.amount, existing.unit),
                );
            }
            Err(_) => *incomplete = true,
        },
    }
}

fn resolve_availability(
    stock: &[&StockItem],
    demand: Option<Quantity>,
    interpretation: MissingStockInterpretation,
) -> Availability {
    if stock.is_empty() {
        return match interpretation {
            MissingStockInterpretation::Absent => Availability::Absent,
            MissingStockInterpretation::Unknown => Availability::Unknown,
        };
    }

    if stock.iter().any(|item| item.level.is_not_tracked()) {
        return Availability::AssumedAvailable;
    }

    let base_unit = stock
        .iter()
        .find_map(|item| item.level.conservative_quantity().map(|q| q.unit));
    let Some(base_unit) = base_unit else {
        return Availability::Unknown;
    };

    let mut on_hand = Decimal::ZERO;
    let mut confidence = Confidence::Exact;
    for item in stock {
        if item.level.is_estimated() {
            confidence = Confidence::Estimated;
        }
        let Some(quantity) = item.level.conservative_quantity() else {
            continue;
        };
        match quantity.convert_to(base_unit) {
            Ok(converted) => on_hand += converted.amount,
            Err(_) => return Availability::Unknown,
        }
    }

    let planned_demand = match demand {
        None => Quantity::new(Decimal::ZERO, base_unit),
        Some(want) => match want.convert_to(base_unit) {
            Ok(converted) => converted,
            Err(_) => return Availability::Unknown,
        },
    };

    Availability::Quantified {
        on_hand: Quantity::new(on_hand, base_unit),
        planned_demand,
        unallocated: Quantity::new(on_hand - planned_demand.amount, base_unit),
        confidence,
    }
}

fn normalise(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn require_revision(id: StockItemId, expected: Revision, actual: Revision) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: STOCK_ITEM,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn commit(outcome: UpdateOutcome, id: StockItemId, expected: Revision) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource: STOCK_ITEM,
            id: id.to_string(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(STOCK_ITEM, id)),
    }
}

#[cfg(test)]
#[path = "stock_tests.rs"]
mod tests;
