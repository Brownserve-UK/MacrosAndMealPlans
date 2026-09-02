use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;

use super::fulfilment::{RecipeFulfilments, RecipeWant, expand_recipe};
use crate::domain::{
    Availability, AvailabilityReport, Confidence, ConsumedAmount, DeductionPlan, DemandGap,
    DemandSubject, HouseholdMemberId, IngredientAvailability, IngredientId, MealItemRef,
    MissingStockInterpretation, NewStockEvent, NewStockItem, ProductAvailability, ProductId,
    Quantity, Recipe, RecipeId, RecipeRequirement, Revision, StockEvent, StockEventKind, StockItem,
    StockItemId, StockItemPatch, UserId, plan_deduction,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, HouseholdMemberRepository, HouseholdSettingsRepository, IngredientRepository,
    MealPlanQuery, MealPlanRepository, MemberQuery, PageRequest, Paginated, ProductRepository,
    RecipeRepository, StockQuery, StockRepository, UpdateOutcome,
};

const STOCK_ITEM: &str = "stock item";

#[derive(Clone)]
pub struct StockService {
    stock: Arc<dyn StockRepository>,
    products: Arc<dyn ProductRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    meal_plans: Arc<dyn MealPlanRepository>,
    recipes: Arc<dyn RecipeRepository>,
    members: Arc<dyn HouseholdMemberRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl StockService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stock: Arc<dyn StockRepository>,
        products: Arc<dyn ProductRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        meal_plans: Arc<dyn MealPlanRepository>,
        recipes: Arc<dyn RecipeRepository>,
        members: Arc<dyn HouseholdMemberRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            stock,
            products,
            ingredients,
            meal_plans,
            recipes,
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

    pub async fn availability_overview(&self, from: Date, to: Date) -> Result<AvailabilityReport> {
        let all = self
            .stock
            .list(&StockQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        let mut ids: Vec<ProductId> = all.items.iter().map(|item| item.product_id).collect();
        sort_dedup(&mut ids, ProductId::as_uuid);
        self.report(&ids, true, from, to).await
    }

    pub async fn availability(
        &self,
        product_ids: &[ProductId],
        from: Date,
        to: Date,
    ) -> Result<AvailabilityReport> {
        self.report(product_ids, false, from, to).await
    }

    async fn report(
        &self,
        product_ids: &[ProductId],
        all_ingredients: bool,
        from: Date,
        to: Date,
    ) -> Result<AvailabilityReport> {
        let interpretation = self.settings.get().await?.missing_stock_interpretation;
        let mut demand = self.planned_demand(from, to).await?;

        if all_ingredients {
            self.seed_pools_from_stock(&mut demand, product_ids).await?;
        }

        let mut ingredient_ids: Vec<IngredientId> = demand
            .ingredients()
            .filter(|id| {
                all_ingredients
                    || demand
                        .pool(id)
                        .iter()
                        .any(|product_id| product_ids.contains(product_id))
            })
            .collect();
        sort_dedup(&mut ingredient_ids, IngredientId::as_uuid);

        let mut wanted: Vec<ProductId> = product_ids.to_vec();
        for ingredient_id in &ingredient_ids {
            wanted.extend(demand.pool(ingredient_id).iter().copied());
        }
        sort_dedup(&mut wanted, ProductId::as_uuid);

        let items = self.stock.list_for_products(&wanted).await?;
        let mut by_product: HashMap<ProductId, Vec<&StockItem>> = HashMap::new();
        for item in &items {
            if item.is_archived() {
                continue;
            }
            by_product.entry(item.product_id).or_default().push(item);
        }

        let names: HashMap<IngredientId, String> = self
            .ingredients
            .get_many(&ingredient_ids)
            .await?
            .into_iter()
            .map(|ingredient| (ingredient.id, ingredient.name))
            .collect();

        let mut apportioned: HashMap<ProductId, Quantity> = HashMap::new();
        let mut ingredients = Vec::with_capacity(ingredient_ids.len());
        for ingredient_id in ingredient_ids {
            let subject = DemandSubject::ingredient(ingredient_id);
            let pool: Vec<&StockItem> = demand
                .pool(&ingredient_id)
                .iter()
                .filter_map(|product_id| by_product.get(product_id))
                .flat_map(|items| items.iter().copied())
                .collect();
            let want = demand.quantity(&subject);

            let owned: Vec<StockItem> = pool.iter().map(|item| (*item).clone()).collect();
            if let Some(want) = want
                && let DeductionPlan::Planned { takes, .. } = plan_deduction(&owned, want)
            {
                for take in takes {
                    let Some(item) = pool.iter().find(|item| item.id == take.stock_item_id) else {
                        continue;
                    };
                    add_quantity(&mut apportioned, item.product_id, take.requested);
                }
            }

            let mut gaps = demand.gaps(&subject);
            let mut pool_want = want;
            for product_id in demand.pool(&ingredient_id) {
                let Some(direct) = demand.quantity(&DemandSubject::product(*product_id)) else {
                    continue;
                };
                match pool_want {
                    None => pool_want = Some(direct),
                    Some(running) => match direct.convert_to(running.unit) {
                        Ok(converted) => {
                            pool_want = Some(Quantity::new(
                                running.amount + converted.amount,
                                running.unit,
                            ));
                        }
                        Err(_) => gaps.push(DemandGap::IncompatibleUnits),
                    },
                }
            }
            gaps.sort_unstable();
            gaps.dedup();

            ingredients.push(IngredientAvailability {
                ingredient_id,
                name: names.get(&ingredient_id).cloned().unwrap_or_default(),
                availability: resolve_availability(&pool, pool_want, interpretation),
                demand_gaps: gaps,
            });
        }

        let mut products = Vec::with_capacity(product_ids.len());
        for &product_id in product_ids {
            let subject = DemandSubject::product(product_id);
            let stock = by_product
                .get(&product_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut gaps = demand.gaps(&subject);
            let mut want = demand.quantity(&subject);
            if let Some(share) = apportioned.get(&product_id).copied() {
                match want {
                    None => want = Some(share),
                    Some(direct) => match share.convert_to(direct.unit) {
                        Ok(converted) => {
                            want =
                                Some(Quantity::new(direct.amount + converted.amount, direct.unit));
                        }
                        Err(_) => gaps.push(DemandGap::IncompatibleUnits),
                    },
                }
            }
            gaps.sort_unstable();
            gaps.dedup();
            products.push(ProductAvailability {
                product_id,
                availability: resolve_availability(stock, want, interpretation),
                demand_gaps: gaps,
            });
        }

        Ok(AvailabilityReport {
            products,
            ingredients,
            demand_gaps: demand.loose_gaps(),
        })
    }

    async fn seed_pools_from_stock(
        &self,
        demand: &mut Demand,
        product_ids: &[ProductId],
    ) -> Result<()> {
        let mut ingredient_ids: Vec<IngredientId> = self
            .products
            .get_many(product_ids)
            .await?
            .into_iter()
            .filter_map(|product| product.mapped_ingredient_id)
            .collect();
        sort_dedup(&mut ingredient_ids, IngredientId::as_uuid);
        if ingredient_ids.is_empty() {
            return Ok(());
        }

        for (ingredient_id, products) in self.products.list_by_ingredient(&ingredient_ids).await? {
            demand.ensure_pool(
                ingredient_id,
                products.into_iter().map(|product| product.id).collect(),
            );
        }
        Ok(())
    }

    async fn planned_demand(&self, from: Date, to: Date) -> Result<Demand> {
        let members = self
            .members
            .list(&MemberQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;

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

        let mut recipe_ids: Vec<RecipeId> = entries
            .iter()
            .flat_map(|entry| entry.components.iter())
            .filter_map(|component| component.item.recipe_id())
            .collect();
        sort_dedup(&mut recipe_ids, RecipeId::as_uuid);
        let recipes: HashMap<RecipeId, Recipe> = self
            .recipes
            .get_many(&recipe_ids)
            .await?
            .into_iter()
            .map(|recipe| (recipe.id, recipe))
            .collect();
        let requirements: Vec<&RecipeRequirement> = recipes
            .values()
            .flat_map(|recipe| recipe.components.iter())
            .map(|component| &component.requirement)
            .collect();
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;

        let mut demand = Demand::default();
        let mut product_cache: HashMap<ProductId, Option<crate::domain::Product>> = HashMap::new();

        for entry in &entries {
            let household = entry.scope == crate::domain::MealPlanScope::Household;
            for component in &entry.components {
                if !component_is_future_demand(entry, component.id) {
                    continue;
                }
                let wanted = if household {
                    demand_amount_for_household(entry, component)
                } else {
                    component.amount
                };

                match component.item {
                    MealItemRef::Product { product_id } => {
                        let subject = DemandSubject::product(product_id);
                        let product = match product_cache.get(&product_id) {
                            Some(cached) => cached.clone(),
                            None => {
                                let loaded = self.products.get(product_id).await?;
                                product_cache.insert(product_id, loaded.clone());
                                loaded
                            }
                        };
                        let Some(product) = product else {
                            demand.note_gap(subject, DemandGap::ProductMissing);
                            continue;
                        };
                        match wanted.resolve(&product) {
                            Ok(quantity) => demand.add(subject, quantity),
                            Err(_) => demand.note_gap(subject, DemandGap::AmountUnresolvable),
                        }
                    }
                    MealItemRef::Recipe { recipe_id } => {
                        let Some(recipe) = recipes.get(&recipe_id) else {
                            demand.note_loose(DemandGap::RecipeMissing);
                            continue;
                        };
                        let ConsumedAmount::Servings(servings) = wanted else {
                            demand.note_loose(DemandGap::AmountUnresolvable);
                            continue;
                        };
                        let expansion = expand_recipe(recipe, servings, &fulfilments);
                        for want in expansion.wants {
                            demand.add_want(&want);
                        }
                        for (subject, gap) in expansion.subject_gaps {
                            demand.note_gap(subject, gap);
                        }
                        for gap in expansion.loose_gaps {
                            demand.note_loose(gap);
                        }
                    }
                }
            }
        }

        Ok(demand)
    }
}

#[derive(Default)]
struct Demand {
    quantities: HashMap<DemandSubject, Quantity>,
    subject_gaps: HashMap<DemandSubject, BTreeSet<DemandGap>>,
    loose: BTreeSet<DemandGap>,
    pools: HashMap<IngredientId, Vec<ProductId>>,
}

impl Demand {
    fn add(&mut self, subject: DemandSubject, quantity: Quantity) {
        match self.quantities.get(&subject).copied() {
            None => {
                self.quantities.insert(subject, quantity);
            }
            Some(existing) => match quantity.convert_to(existing.unit) {
                Ok(converted) => {
                    self.quantities.insert(
                        subject,
                        Quantity::new(existing.amount + converted.amount, existing.unit),
                    );
                }
                Err(_) => self.note_gap(subject, DemandGap::IncompatibleUnits),
            },
        }
    }

    fn add_want(&mut self, want: &RecipeWant) {
        if let Some(ingredient_id) = want.target.subject.ingredient_id() {
            self.pools
                .entry(ingredient_id)
                .or_insert_with(|| want.target.product_ids.clone());
        }
        self.add(want.target.subject, want.want);
    }

    fn note_gap(&mut self, subject: DemandSubject, gap: DemandGap) {
        if let Some(ingredient_id) = subject.ingredient_id() {
            self.pools.entry(ingredient_id).or_default();
        }
        self.subject_gaps.entry(subject).or_default().insert(gap);
    }

    fn note_loose(&mut self, gap: DemandGap) {
        self.loose.insert(gap);
    }

    fn quantity(&self, subject: &DemandSubject) -> Option<Quantity> {
        self.quantities.get(subject).copied()
    }

    fn gaps(&self, subject: &DemandSubject) -> Vec<DemandGap> {
        self.subject_gaps
            .get(subject)
            .map(|gaps| gaps.iter().copied().collect())
            .unwrap_or_default()
    }

    fn loose_gaps(&self) -> Vec<DemandGap> {
        self.loose.iter().copied().collect()
    }

    fn ensure_pool(&mut self, ingredient_id: IngredientId, product_ids: Vec<ProductId>) {
        let pool = self.pools.entry(ingredient_id).or_default();
        pool.extend(product_ids);
        sort_dedup(pool, ProductId::as_uuid);
    }

    fn pool(&self, ingredient_id: &IngredientId) -> &[ProductId] {
        self.pools
            .get(ingredient_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn ingredients(&self) -> impl Iterator<Item = IngredientId> + '_ {
        self.pools.keys().copied()
    }
}

fn sort_dedup<T: Copy, K: Ord>(ids: &mut Vec<T>, key: impl Fn(&T) -> K) {
    ids.sort_unstable_by_key(&key);
    ids.dedup_by_key(|id| key(id));
}

fn add_quantity(totals: &mut HashMap<ProductId, Quantity>, key: ProductId, quantity: Quantity) {
    match totals.get(&key).copied() {
        None => {
            totals.insert(key, quantity);
        }
        Some(existing) => {
            if let Ok(converted) = quantity.convert_to(existing.unit) {
                totals.insert(
                    key,
                    Quantity::new(existing.amount + converted.amount, existing.unit),
                );
            }
        }
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
