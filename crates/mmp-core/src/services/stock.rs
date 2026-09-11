use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;

use super::fulfilment::{RecipeFulfilments, RecipeWant, expand_recipe};
use super::revision::{commit_outcome, require_revision};
use crate::domain::{
    Availability, AvailabilityReport, Confidence, ConsumedAmount, CookedFoodAvailability,
    DeductionCandidates, DeductionPlan, DemandClaim, DemandGap, DemandSubject, HouseholdMemberId,
    IngredientAvailability, IngredientId, MealItemRef, MissingStock, NewStockEvent, NewStockItem,
    PreparedMealAvailability, PreparedMealId, ProductAvailability, ProductId, Quantity, Recipe,
    RecipeId, RecipeRequirement, Revision, StockEvent, StockEventKind, StockItem, StockItemId,
    StockItemPatch, StockLevel, Unit, UserId, apply_take, plan_deduction,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, HouseholdMemberRepository, HouseholdSettingsRepository, IngredientRepository,
    MealPlanQuery, MealPlanRepository, MemberQuery, PageRequest, Paginated,
    PreparedBatchRepository, PreparedMealRepository, ProductRepository, RecipeRepository,
    StockQuery, StockRepository,
};

const STOCK_ITEM: &str = "stock item";

type ProductTrackingByMapping =
    HashMap<ProductId, (Option<bool>, Option<IngredientId>, Option<PreparedMealId>)>;

#[derive(Clone)]
pub(crate) struct StockSnapshot {
    pub report: AvailabilityReport,
    pub pools: HashMap<IngredientId, Vec<ProductId>>,
    pub prepared_meal_pools: HashMap<PreparedMealId, Vec<ProductId>>,
    pub items: Vec<StockItem>,
}

#[derive(Clone)]
pub struct StockService {
    stock: Arc<dyn StockRepository>,
    products: Arc<dyn ProductRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    prepared_meals: Arc<dyn PreparedMealRepository>,
    meal_plans: Arc<dyn MealPlanRepository>,
    recipes: Arc<dyn RecipeRepository>,
    batches: Arc<dyn PreparedBatchRepository>,
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
        prepared_meals: Arc<dyn PreparedMealRepository>,
        meal_plans: Arc<dyn MealPlanRepository>,
        recipes: Arc<dyn RecipeRepository>,
        batches: Arc<dyn PreparedBatchRepository>,
        members: Arc<dyn HouseholdMemberRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            stock,
            products,
            ingredients,
            prepared_meals,
            meal_plans,
            recipes,
            batches,
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
        if let Some(product_id) = input.subject.product_id() {
            let product = self
                .products
                .get(product_id)
                .await?
                .ok_or_else(|| CoreError::not_found("product", product_id))?;
            if product.is_archived() {
                return Err(CoreError::conflict("That product is archived."));
            }
        }

        let now = self.clock.now();
        let item = StockItem {
            id: StockItemId::new(),
            subject: input.subject,
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
        require_revision(STOCK_ITEM, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        let previous_mode = current.tracking_mode();
        let previous_level = current.level;
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
            quantity_delta: level_delta(previous_level, current.level),
            actor_user_id: Some(actor),
            subject_member_id: subject,
            source: None,
            reverses_event_id: None,
            note: None,
        };
        commit_outcome(
            STOCK_ITEM,
            id,
            expected,
            self.stock.update(&current, expected, &event).await?,
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
        require_revision(STOCK_ITEM, id, expected, current.revision)?;
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
        commit_outcome(
            STOCK_ITEM,
            id,
            expected,
            self.stock.update(&current, expected, &event).await?,
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
        let mut ids: Vec<ProductId> = all
            .items
            .iter()
            .filter_map(|item| item.product_id())
            .collect();
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

    pub(crate) async fn snapshot(&self, from: Date, to: Date) -> Result<StockSnapshot> {
        let demand = self.planned_demand(from, to).await?;

        let mut ids: Vec<ProductId> = demand
            .quantities
            .keys()
            .filter_map(|subject| subject.product_id())
            .collect();
        for ingredient_id in demand.ingredients() {
            ids.extend(demand.pool(&ingredient_id).iter().copied());
        }
        let held = self
            .stock
            .list(&StockQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        ids.extend(held.items.iter().filter_map(|item| item.product_id()));
        sort_dedup(&mut ids, ProductId::as_uuid);

        self.build_snapshot(demand, &ids, true).await
    }

    async fn report(
        &self,
        product_ids: &[ProductId],
        all_ingredients: bool,
        from: Date,
        to: Date,
    ) -> Result<AvailabilityReport> {
        let demand = self.planned_demand(from, to).await?;
        let mut snapshot = self
            .build_snapshot(demand, product_ids, all_ingredients)
            .await?;
        if all_ingredients {
            snapshot.report.claims = Vec::new();
        }
        Ok(snapshot.report)
    }

    async fn build_snapshot(
        &self,
        mut demand: Demand,
        product_ids: &[ProductId],
        all_ingredients: bool,
    ) -> Result<StockSnapshot> {
        let interpretation = self.settings.get().await?.missing_stock_interpretation;

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

        let mut prepared_meal_ids: Vec<PreparedMealId> = demand
            .prepared_meals()
            .filter(|id| {
                all_ingredients
                    || demand
                        .prepared_meal_pool(id)
                        .iter()
                        .any(|product_id| product_ids.contains(product_id))
            })
            .collect();
        sort_dedup(&mut prepared_meal_ids, PreparedMealId::as_uuid);

        let claims: Vec<DemandClaim> = demand
            .claims
            .iter()
            .filter(|claim| match claim.subject {
                DemandSubject::Product { product_id } => product_ids.contains(&product_id),
                DemandSubject::Ingredient { ingredient_id } => {
                    ingredient_ids.contains(&ingredient_id)
                }
                DemandSubject::PreparedMeal { prepared_meal_id } => {
                    prepared_meal_ids.contains(&prepared_meal_id)
                }
                DemandSubject::PreparedPortion { .. } => false,
                DemandSubject::CookedFood { .. } => all_ingredients,
            })
            .cloned()
            .collect();

        let mut wanted: Vec<ProductId> = product_ids.to_vec();
        for ingredient_id in &ingredient_ids {
            wanted.extend(demand.pool(ingredient_id).iter().copied());
        }
        for prepared_meal_id in &prepared_meal_ids {
            wanted.extend(demand.prepared_meal_pool(prepared_meal_id).iter().copied());
        }
        sort_dedup(&mut wanted, ProductId::as_uuid);

        let items = self.stock.list_for_products(&wanted).await?;
        let mut by_product: HashMap<ProductId, Vec<&StockItem>> = HashMap::new();
        for item in &items {
            if item.is_archived() {
                continue;
            }
            let Some(product_id) = item.product_id() else {
                continue;
            };
            by_product.entry(product_id).or_default().push(item);
        }

        let catalogue = self.products.get_many(&wanted).await?;
        let product_tracking: ProductTrackingByMapping = catalogue
            .iter()
            .map(|product| {
                (
                    product.id,
                    (
                        product.track_stock,
                        product.mapped_ingredient_id,
                        product.mapped_prepared_meal_id,
                    ),
                )
            })
            .collect();

        let mut tracked_ingredient_ids = ingredient_ids.clone();
        tracked_ingredient_ids.extend(catalogue.iter().filter_map(|p| p.mapped_ingredient_id));
        sort_dedup(&mut tracked_ingredient_ids, IngredientId::as_uuid);

        let catalogue_ingredients = self.ingredients.get_many(&tracked_ingredient_ids).await?;
        let ingredient_tracking: HashMap<IngredientId, Option<bool>> = catalogue_ingredients
            .iter()
            .map(|ingredient| (ingredient.id, ingredient.track_stock))
            .collect();
        let names: HashMap<IngredientId, String> = catalogue_ingredients
            .into_iter()
            .map(|ingredient| (ingredient.id, ingredient.name))
            .collect();

        let mut tracked_prepared_meal_ids = prepared_meal_ids.clone();
        tracked_prepared_meal_ids
            .extend(catalogue.iter().filter_map(|p| p.mapped_prepared_meal_id));
        sort_dedup(&mut tracked_prepared_meal_ids, PreparedMealId::as_uuid);

        let catalogue_prepared_meals = self
            .prepared_meals
            .get_many(&tracked_prepared_meal_ids)
            .await?;
        let prepared_meal_tracking: HashMap<PreparedMealId, Option<bool>> =
            catalogue_prepared_meals
                .iter()
                .map(|prepared_meal| (prepared_meal.id, prepared_meal.track_stock))
                .collect();
        let prepared_meal_names: HashMap<PreparedMealId, String> = catalogue_prepared_meals
            .into_iter()
            .map(|prepared_meal| (prepared_meal.id, prepared_meal.name))
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

            let free = free_pool_stock(&demand, demand.pool(&ingredient_id), &by_product);
            if let Some(want) = want
                && let DeductionPlan::Planned { takes, .. } = plan_deduction(&free, want)
            {
                for take in takes {
                    let Some(item) = pool.iter().find(|item| item.id == take.stock_item_id) else {
                        continue;
                    };
                    let Some(product_id) = item.product_id() else {
                        continue;
                    };
                    add_quantity(&mut apportioned, product_id, take.requested);
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
                availability: resolve_availability(
                    &pool,
                    pool_want,
                    MissingStock::resolve(
                        None,
                        ingredient_tracking.get(&ingredient_id).copied().flatten(),
                        interpretation,
                    ),
                ),
                demand_gaps: gaps,
            });
        }

        let mut prepared_meals = Vec::with_capacity(prepared_meal_ids.len());
        for prepared_meal_id in prepared_meal_ids {
            let subject = DemandSubject::prepared_meal(prepared_meal_id);
            let pool: Vec<&StockItem> = demand
                .prepared_meal_pool(&prepared_meal_id)
                .iter()
                .filter_map(|product_id| by_product.get(product_id))
                .flat_map(|items| items.iter().copied())
                .collect();
            let want = demand.quantity(&subject);

            let free = free_pool_stock(
                &demand,
                demand.prepared_meal_pool(&prepared_meal_id),
                &by_product,
            );
            if let Some(want) = want
                && let DeductionPlan::Planned { takes, .. } = plan_deduction(&free, want)
            {
                for take in takes {
                    let Some(item) = pool.iter().find(|item| item.id == take.stock_item_id) else {
                        continue;
                    };
                    let Some(product_id) = item.product_id() else {
                        continue;
                    };
                    add_quantity(&mut apportioned, product_id, take.requested);
                }
            }

            let mut gaps = demand.gaps(&subject);
            let mut pool_want = want;
            for product_id in demand.prepared_meal_pool(&prepared_meal_id) {
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

            prepared_meals.push(PreparedMealAvailability {
                prepared_meal_id,
                name: prepared_meal_names
                    .get(&prepared_meal_id)
                    .cloned()
                    .unwrap_or_default(),
                availability: resolve_availability(
                    &pool,
                    pool_want,
                    MissingStock::resolve(
                        None,
                        prepared_meal_tracking
                            .get(&prepared_meal_id)
                            .copied()
                            .flatten(),
                        interpretation,
                    ),
                ),
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
            let (own, mapped_ingredient, mapped_prepared_meal) = product_tracking
                .get(&product_id)
                .copied()
                .unwrap_or((None, None, None));
            let inherited = mapped_ingredient
                .and_then(|id| ingredient_tracking.get(&id).copied())
                .flatten()
                .or_else(|| {
                    mapped_prepared_meal
                        .and_then(|id| prepared_meal_tracking.get(&id).copied())
                        .flatten()
                });
            products.push(ProductAvailability {
                product_id,
                availability: resolve_availability(
                    stock,
                    want,
                    MissingStock::resolve(own, inherited, interpretation),
                ),
                demand_gaps: gaps,
            });
        }

        let mut pools: HashMap<IngredientId, Vec<ProductId>> = HashMap::new();
        for ingredient in &ingredients {
            pools.insert(
                ingredient.ingredient_id,
                demand.pool(&ingredient.ingredient_id).to_vec(),
            );
        }

        let mut prepared_meal_pools: HashMap<PreparedMealId, Vec<ProductId>> = HashMap::new();
        for prepared_meal in &prepared_meals {
            prepared_meal_pools.insert(
                prepared_meal.prepared_meal_id,
                demand
                    .prepared_meal_pool(&prepared_meal.prepared_meal_id)
                    .to_vec(),
            );
        }

        let cooked_food = if all_ingredients {
            self.cooked_availability(&demand).await?
        } else {
            Vec::new()
        };

        Ok(StockSnapshot {
            report: AvailabilityReport {
                products,
                ingredients,
                prepared_meals,
                cooked_food,
                demand_gaps: demand.loose_gaps(),
                claims,
            },
            pools,
            prepared_meal_pools,
            items,
        })
    }

    async fn cooked_availability(&self, demand: &Demand) -> Result<Vec<CookedFoodAvailability>> {
        let held = self
            .stock
            .list(&StockQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        let portions: Vec<StockItem> = held
            .items
            .into_iter()
            .filter(|item| item.prepared_batch_id().is_some())
            .collect();

        let mut batch_ids: Vec<_> = portions
            .iter()
            .filter_map(|item| item.prepared_batch_id())
            .collect();
        sort_dedup(&mut batch_ids, |id| id.as_uuid());
        let batches = self.batches.get_many(&batch_ids).await?;
        let batch_recipe: HashMap<_, _> = batches
            .iter()
            .filter_map(|batch| batch.recipe_id.map(|recipe_id| (batch.id, recipe_id)))
            .collect();

        let mut by_recipe: HashMap<RecipeId, Vec<&StockItem>> = HashMap::new();
        for item in &portions {
            let Some(recipe_id) = item
                .prepared_batch_id()
                .and_then(|id| batch_recipe.get(&id).copied())
            else {
                continue;
            };
            by_recipe.entry(recipe_id).or_default().push(item);
        }

        let mut recipe_ids: Vec<RecipeId> = by_recipe.keys().copied().collect();
        recipe_ids.extend(
            demand
                .quantities
                .keys()
                .filter_map(DemandSubject::cooked_recipe_id),
        );
        sort_dedup(&mut recipe_ids, RecipeId::as_uuid);

        let names: HashMap<RecipeId, String> = self
            .recipes
            .get_many(&recipe_ids)
            .await?
            .into_iter()
            .map(|recipe| (recipe.id, recipe.name))
            .collect();

        let mut cooked: Vec<CookedFoodAvailability> = recipe_ids
            .into_iter()
            .map(|recipe_id| {
                let pool = by_recipe.remove(&recipe_id).unwrap_or_default();
                let want = demand.quantity(&DemandSubject::cooked_food(recipe_id));
                CookedFoodAvailability {
                    recipe_id,
                    name: names.get(&recipe_id).cloned().unwrap_or_default(),
                    availability: resolve_availability(&pool, want, MissingStock::Absent),
                }
            })
            .collect();
        cooked.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cooked)
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

        let mut prepared_meal_ids: Vec<PreparedMealId> = self
            .products
            .get_many(product_ids)
            .await?
            .into_iter()
            .filter_map(|product| product.mapped_prepared_meal_id)
            .collect();
        sort_dedup(&mut prepared_meal_ids, PreparedMealId::as_uuid);
        if prepared_meal_ids.is_empty() {
            return Ok(());
        }

        for (prepared_meal_id, products) in self
            .products
            .list_by_prepared_meal(&prepared_meal_ids)
            .await?
        {
            demand.ensure_prepared_meal_pool(
                prepared_meal_id,
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

        let settings = self.settings.get().await?;
        let assumption_rules = crate::domain::AssumptionRules {
            now: self.clock.now(),
            meal_times: settings.meal_times,
            enabled: settings.assume_eaten_when_time_passes,
        };

        let component_ids: Vec<crate::domain::MealPlanComponentId> = entries
            .iter()
            .flat_map(|entry| entry.components.iter())
            .map(|component| component.id)
            .collect();
        let prepared = self.batches.for_components(&component_ids).await?;

        let mut demand = Demand::default();
        let mut product_cache: HashMap<ProductId, Option<crate::domain::Product>> = HashMap::new();

        for entry in &entries {
            let assumed = crate::domain::Assumption::for_entry(
                entry,
                assumption_rules.now,
                &assumption_rules.meal_times,
                assumption_rules.enabled,
            )
            .assumed;
            for component in &entry.components {
                let Some(wanted) =
                    unresolved_demand(entry, component, prepared.contains_key(&component.id))
                else {
                    continue;
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
                            Ok(quantity) => {
                                demand.add(subject, quantity);
                                demand.note_claim(entry, subject, quantity, None, assumed);
                            }
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
                            demand.note_claim(
                                entry,
                                want.target.subject,
                                want.want,
                                Some(recipe.name.clone()),
                                assumed,
                            );
                        }
                        for (subject, gap) in expansion.subject_gaps {
                            demand.note_gap(subject, gap);
                        }
                        for gap in expansion.loose_gaps {
                            demand.note_loose(gap);
                        }
                    }
                    MealItemRef::Dish { recipe_id } => {
                        let ConsumedAmount::Servings(servings) = wanted else {
                            demand.note_loose(DemandGap::AmountUnresolvable);
                            continue;
                        };
                        let subject = DemandSubject::cooked_food(recipe_id);
                        let quantity = Quantity::new(servings, Unit::Serving);
                        demand.add(subject, quantity);
                        demand.note_claim(entry, subject, quantity, None, assumed);
                    }
                    MealItemRef::Ingredient { ingredient_id } => {
                        let subject = DemandSubject::ingredient(ingredient_id);
                        let ConsumedAmount::Measure(quantity) = wanted else {
                            demand.note_gap(subject, DemandGap::AmountUnresolvable);
                            continue;
                        };
                        let pool = self
                            .products
                            .list_by_ingredient(&[ingredient_id])
                            .await?
                            .remove(&ingredient_id)
                            .unwrap_or_default();
                        demand.ensure_pool(
                            ingredient_id,
                            pool.iter().map(|product| product.id).collect(),
                        );
                        if pool.is_empty() {
                            demand.note_gap(subject, DemandGap::FoodHasNoProducts);
                        }
                        demand.add(subject, quantity);
                        demand.note_claim(entry, subject, quantity, None, assumed);
                    }
                    MealItemRef::PreparedMeal { prepared_meal_id } => {
                        let subject = DemandSubject::prepared_meal(prepared_meal_id);
                        let ConsumedAmount::Measure(quantity) = wanted else {
                            demand.note_gap(subject, DemandGap::AmountUnresolvable);
                            continue;
                        };
                        let pool = self
                            .products
                            .list_by_prepared_meal(&[prepared_meal_id])
                            .await?
                            .remove(&prepared_meal_id)
                            .unwrap_or_default();
                        demand.ensure_prepared_meal_pool(
                            prepared_meal_id,
                            pool.iter().map(|product| product.id).collect(),
                        );
                        if pool.is_empty() {
                            demand.note_gap(subject, DemandGap::FoodHasNoProducts);
                        }
                        demand.add(subject, quantity);
                        demand.note_claim(entry, subject, quantity, None, assumed);
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
    prepared_meal_pools: HashMap<PreparedMealId, Vec<ProductId>>,
    claims: Vec<DemandClaim>,
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
        if let Some(ingredient_id) = want.target.subject.ingredient_id()
            && let DeductionCandidates::Products(product_ids) = &want.target.candidates
        {
            self.pools
                .entry(ingredient_id)
                .or_insert_with(|| product_ids.clone());
        }
        self.add(want.target.subject, want.want);
    }

    fn note_gap(&mut self, subject: DemandSubject, gap: DemandGap) {
        if let Some(ingredient_id) = subject.ingredient_id() {
            self.pools.entry(ingredient_id).or_default();
        }
        if let Some(prepared_meal_id) = subject.prepared_meal_id() {
            self.prepared_meal_pools
                .entry(prepared_meal_id)
                .or_default();
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

    fn ensure_prepared_meal_pool(
        &mut self,
        prepared_meal_id: PreparedMealId,
        product_ids: Vec<ProductId>,
    ) {
        let pool = self
            .prepared_meal_pools
            .entry(prepared_meal_id)
            .or_default();
        pool.extend(product_ids);
        sort_dedup(pool, ProductId::as_uuid);
    }

    fn prepared_meal_pool(&self, prepared_meal_id: &PreparedMealId) -> &[ProductId] {
        self.prepared_meal_pools
            .get(prepared_meal_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn note_claim(
        &mut self,
        entry: &crate::domain::MealPlanEntry,
        subject: DemandSubject,
        quantity: Quantity,
        recipe_name: Option<String>,
        assumed: bool,
    ) {
        self.claims.push(DemandClaim {
            subject,
            quantity,
            entry_id: entry.id,
            planned_on: entry.planned_on,
            slot: entry.slot,
            scope: entry.scope,
            recipe_name,
            assumed,
        });
    }

    fn ingredients(&self) -> impl Iterator<Item = IngredientId> + '_ {
        self.pools.keys().copied()
    }

    fn prepared_meals(&self) -> impl Iterator<Item = PreparedMealId> + '_ {
        self.prepared_meal_pools.keys().copied()
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

fn unresolved_demand(
    entry: &crate::domain::MealPlanEntry,
    component: &crate::domain::MealPlanComponent,
    prepared: bool,
) -> Option<crate::domain::ConsumedAmount> {
    use crate::domain::ParticipantStatus;

    if prepared {
        return None;
    }

    let mut settled: Vec<crate::domain::ConsumedAmount> = Vec::new();

    for allocation in entry
        .participants
        .iter()
        .flat_map(|participant| participant.allocations.iter())
        .filter(|allocation| allocation.component_id == component.id)
        .filter(|allocation| allocation.status != ParticipantStatus::Planned)
    {
        settled.push(allocation.allocated);
    }

    for group in &entry.guest_groups {
        for allocation in group
            .allocations
            .iter()
            .filter(|allocation| allocation.component_id == component.id)
            .filter(|allocation| allocation.status != ParticipantStatus::Planned)
        {
            for _ in 0..group.count.max(0) {
                settled.push(allocation.allocated);
            }
        }
    }

    let remaining =
        crate::domain::forecast_remaining(&component.amount, &settled).unwrap_or(component.amount);
    (remaining.value() > rust_decimal::Decimal::ZERO).then_some(remaining)
}

fn free_pool_stock(
    demand: &Demand,
    pool: &[ProductId],
    by_product: &HashMap<ProductId, Vec<&StockItem>>,
) -> Vec<StockItem> {
    let mut free = Vec::new();
    for product_id in pool {
        let Some(items) = by_product.get(product_id) else {
            continue;
        };
        let mut owned: Vec<StockItem> = items.iter().map(|item| (*item).clone()).collect();
        if let Some(pinned) = demand.quantity(&DemandSubject::product(*product_id))
            && let DeductionPlan::Planned { takes, .. } = plan_deduction(&owned, pinned)
        {
            for take in takes {
                if let Some(item) = owned.iter_mut().find(|item| item.id == take.stock_item_id)
                    && let Some(applied) = apply_take(&item.level, take.requested)
                {
                    item.level = applied.new_level;
                }
            }
        }
        free.extend(owned);
    }
    free
}

fn resolve_availability(
    stock: &[&StockItem],
    demand: Option<Quantity>,
    missing: MissingStock,
) -> Availability {
    if stock.is_empty() {
        return match missing {
            MissingStock::Absent => Availability::Absent,
            MissingStock::Unknown => Availability::Unknown,
            MissingStock::Assumed => Availability::AssumedAvailable,
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

fn level_delta(before: StockLevel, after: StockLevel) -> Option<Quantity> {
    match (
        before.conservative_quantity(),
        after.conservative_quantity(),
    ) {
        (Some(before), Some(after)) => {
            let before = before.convert_to(after.unit).ok()?;
            Some(Quantity::new(after.amount - before.amount, after.unit))
        }
        (None, Some(after)) => Some(after),
        (Some(before), None) => Some(Quantity::new(-before.amount, before.unit)),
        (None, None) => None,
    }
}

#[cfg(test)]
#[path = "stock_tests.rs"]
mod tests;
