use std::collections::HashMap;
use std::sync::Arc;

use time::{Date, Duration};

use super::revision::commit_outcome;
use crate::domain::{
    Assignment, Availability, Certainty, DemandClaim, DemandGap, DemandSubject, ExceptionState,
    IngredientId, NewPurchase, NewShoppingCadence, NewShoppingListItem, NewStockEvent,
    NewStockItem, OpportunityException, PreparedMealId, ProductId, Purchase, PurchaseId,
    PurchasePatch, PurchaseState, Quantity, Revision, ShoppingCadence, ShoppingListItem,
    ShoppingListItemId, ShoppingListItemPatch, ShoppingOpportunity, ShoppingOpportunityId,
    ShoppingRequirement, ShoppingSection, ShoppingTrip, ShoppingTripId, ShoppingTripRow,
    ShoppingTripRowId, StockEffectSource, StockEventKind, StockEventSource, StockItem, StockLevel,
    StockSubject, StorageLocation, SuggestionReason, TripState, UncoveredClaim, UserId, assign,
    cover, expand_opportunities,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, FinishedPurchase, HouseholdSettingsRepository, IngredientRepository,
    NewStockFromPurchase, Paginated, PreparedMealRepository, ProductRepository, PurchaseQuery,
    PurchaseRepository, ShoppingCadenceRepository, ShoppingListItemRepository,
    ShoppingOpportunityRepository, ShoppingTripRepository, UpdateOutcome,
};
use crate::services::StockService;

const CADENCE: &str = "shopping cadence";
const OPPORTUNITY: &str = "shopping opportunity";
const PURCHASE: &str = "purchase";
const LIST_ITEM: &str = "shopping list item";

const OPPORTUNITY_LOOKAHEAD_DAYS: i64 = 70;

const PLANNING_WINDOW_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FinishedShop {
    pub stocked: usize,
    pub still_pending: usize,
}

#[derive(Debug, Clone)]
pub struct ShoppingList {
    pub opportunities: Vec<ShoppingOpportunity>,
    pub focus: Option<Date>,
    pub requirements: Vec<ShoppingRequirement>,
    pub manual: Vec<ShoppingListItem>,
    pub unplanned: Vec<Purchase>,
    pub trip: Option<ShoppingTrip>,
    pub counts: Vec<ShopCount>,
    pub cadence_configured: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopCount {
    pub date: Date,
    pub items: usize,
}

#[derive(Clone)]
pub struct ShoppingService {
    cadence: Arc<dyn ShoppingCadenceRepository>,
    opportunities: Arc<dyn ShoppingOpportunityRepository>,
    purchases: Arc<dyn PurchaseRepository>,
    list_items: Arc<dyn ShoppingListItemRepository>,
    trips: Arc<dyn ShoppingTripRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    prepared_meals: Arc<dyn PreparedMealRepository>,
    products: Arc<dyn ProductRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    stock: StockService,
    clock: Arc<dyn Clock>,
}

impl ShoppingService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cadence: Arc<dyn ShoppingCadenceRepository>,
        opportunities: Arc<dyn ShoppingOpportunityRepository>,
        purchases: Arc<dyn PurchaseRepository>,
        list_items: Arc<dyn ShoppingListItemRepository>,
        trips: Arc<dyn ShoppingTripRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        prepared_meals: Arc<dyn PreparedMealRepository>,
        products: Arc<dyn ProductRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        stock: StockService,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            cadence,
            opportunities,
            purchases,
            list_items,
            trips,
            ingredients,
            prepared_meals,
            products,
            settings,
            stock,
            clock,
        }
    }

    pub async fn trip(&self, date: Date) -> Result<Option<ShoppingTrip>> {
        self.trips.for_date(date).await
    }

    pub async fn start_shop(&self, date: Date, actor: UserId) -> Result<ShoppingTrip> {
        if let Some(existing) = self.trips.for_date(date).await? {
            return Ok(existing);
        }

        let list = self.requirements(Some(date)).await?;
        let mut rows: Vec<ShoppingTripRow> = list
            .requirements
            .iter()
            .map(|requirement| ShoppingTripRow {
                id: ShoppingTripRowId::new(),
                ingredient_id: requirement.subject.ingredient_id(),
                prepared_meal_id: requirement.subject.prepared_meal_id(),
                product_id: requirement.subject.product_id(),
                name: requirement.name.clone(),
                quantity: requirement.quantity,
                section: Some(requirement.section),
            })
            .collect();
        rows.extend(list.manual.iter().map(|item| ShoppingTripRow {
            id: ShoppingTripRowId::new(),
            ingredient_id: item.ingredient_id,
            prepared_meal_id: item.prepared_meal_id,
            product_id: item.product_id,
            name: item.name.clone(),
            quantity: item.quantity,
            section: item.section,
        }));

        let now = self.clock.now();
        let trip = ShoppingTrip {
            id: ShoppingTripId::new(),
            opportunity_date: date,
            state: TripState::Shopping,
            started_at: now,
            finished_at: None,
            started_by: actor,
            rows,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        self.trips.insert(&trip).await?;
        Ok(trip)
    }

    pub async fn list_items(&self) -> Result<Vec<ShoppingListItem>> {
        self.list_items.list().await
    }

    pub async fn add_list_item(
        &self,
        input: NewShoppingListItem,
        actor: UserId,
    ) -> Result<ShoppingListItem> {
        input.validate()?;
        let now = self.clock.now();
        let item = ShoppingListItem {
            id: ShoppingListItemId::new(),
            ingredient_id: input.ingredient_id,
            prepared_meal_id: input.prepared_meal_id,
            product_id: input.product_id,
            name: input.name.trim().to_owned(),
            quantity: input.quantity,
            section: input.section,
            opportunity_date: input.opportunity_date,
            created_by: actor,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        self.list_items.insert(&item).await?;
        Ok(item)
    }

    pub async fn update_list_item(
        &self,
        id: ShoppingListItemId,
        expected: Revision,
        patch: ShoppingListItemPatch,
    ) -> Result<ShoppingListItem> {
        patch.validate()?;
        let Some(mut item) = self.list_items.get(id).await? else {
            return Err(CoreError::not_found(LIST_ITEM, id.to_string()));
        };
        if let Some(name) = patch.name {
            item.name = name.trim().to_owned();
        }
        item.quantity = patch.quantity.apply(item.quantity);
        item.section = patch.section.apply(item.section);
        item.opportunity_date = patch.opportunity_date.apply(item.opportunity_date);
        item.updated_at = self.clock.now();
        item.revision = expected.next();

        let outcome = self.list_items.update(&item, expected).await?;
        commit_outcome(LIST_ITEM, id, expected, outcome)?;
        Ok(item)
    }

    pub async fn remove_list_item(&self, id: ShoppingListItemId, expected: Revision) -> Result<()> {
        let outcome = self.list_items.delete(id, expected).await?;
        commit_outcome(LIST_ITEM, id, expected, outcome)
    }

    pub async fn cadence(&self) -> Result<Option<ShoppingCadence>> {
        self.cadence.get().await
    }

    pub async fn set_cadence(
        &self,
        expected: Revision,
        input: NewShoppingCadence,
    ) -> Result<ShoppingCadence> {
        input.validate()?;
        let now = self.clock.now();
        let existing = self.cadence.get().await?;
        let cadence = ShoppingCadence {
            interval_weeks: input.interval_weeks,
            days: input.days,
            anchor: input.anchor,
            usual_time: input.usual_time,
            revision: existing
                .as_ref()
                .map(|current| current.revision.next())
                .unwrap_or(Revision::INITIAL),
            created_at: existing.map(|current| current.created_at).unwrap_or(now),
            updated_at: now,
        };
        let outcome = self.cadence.set(&cadence, expected).await?;
        commit_outcome(CADENCE, "singleton", expected, outcome)?;
        Ok(cadence)
    }

    pub async fn clear_cadence(&self, expected: Revision) -> Result<()> {
        let outcome = self.cadence.clear(expected).await?;
        commit_outcome(CADENCE, "singleton", expected, outcome)
    }

    pub async fn opportunities(&self, from: Date, to: Date) -> Result<Vec<ShoppingOpportunity>> {
        let cadence = self.cadence.get().await?;
        let exceptions = self.opportunities.list_in_range(from, to).await?;
        Ok(expand_opportunities(
            cadence.as_ref(),
            &exceptions,
            from,
            to,
        ))
    }

    pub async fn upcoming(&self) -> Result<Vec<ShoppingOpportunity>> {
        let today = self.clock.now().date();
        self.opportunities(today, today + Duration::days(OPPORTUNITY_LOOKAHEAD_DAYS))
            .await
    }

    pub async fn move_opportunity(
        &self,
        occurrence: Date,
        to: Date,
        expected: Revision,
    ) -> Result<()> {
        self.record_exception(
            ExceptionState::Moved,
            Some(occurrence),
            Some(to),
            None,
            Some(expected),
        )
        .await
    }

    pub async fn skip_opportunity(&self, occurrence: Date, expected: Revision) -> Result<()> {
        self.record_exception(
            ExceptionState::Skipped,
            Some(occurrence),
            None,
            None,
            Some(expected),
        )
        .await
    }

    pub async fn add_one_off(&self, date: Date, note: Option<String>) -> Result<()> {
        self.record_exception(ExceptionState::OneOff, None, Some(date), note, None)
            .await
    }

    pub async fn restore_opportunity(&self, occurrence: Date) -> Result<()> {
        let Some(existing) = self.opportunities.find_for_occurrence(occurrence).await? else {
            return Err(CoreError::not_found(OPPORTUNITY, occurrence.to_string()));
        };
        match self.opportunities.delete(existing.id).await? {
            UpdateOutcome::Updated => Ok(()),
            _ => Err(CoreError::not_found(OPPORTUNITY, occurrence.to_string())),
        }
    }

    pub async fn delete_opportunity(&self, id: ShoppingOpportunityId) -> Result<()> {
        match self.opportunities.delete(id).await? {
            UpdateOutcome::Updated => Ok(()),
            _ => Err(CoreError::not_found(OPPORTUNITY, id.to_string())),
        }
    }

    async fn record_exception(
        &self,
        state: ExceptionState,
        generated_for: Option<Date>,
        effective_date: Option<Date>,
        note: Option<String>,
        expected: Option<Revision>,
    ) -> Result<()> {
        let now = self.clock.now();
        let existing = match generated_for {
            Some(occurrence) => self.opportunities.find_for_occurrence(occurrence).await?,
            None => None,
        };
        if let Some(expected) = expected {
            let actual = existing
                .as_ref()
                .map(|current| current.revision)
                .unwrap_or(Revision::UNRECORDED);
            if actual != expected {
                return Err(CoreError::RevisionMismatch {
                    resource: OPPORTUNITY,
                    id: generated_for
                        .map(|date| date.to_string())
                        .unwrap_or_default(),
                    expected,
                    actual,
                });
            }
        }
        let write_expected = existing
            .as_ref()
            .map(|current| current.revision)
            .unwrap_or(Revision::UNRECORDED);
        let exception = OpportunityException {
            id: existing
                .as_ref()
                .map(|current| current.id)
                .unwrap_or_else(ShoppingOpportunityId::new),
            generated_for,
            effective_date,
            usual_time: existing.as_ref().and_then(|current| current.usual_time),
            state,
            note,
            revision: existing
                .as_ref()
                .map(|current| current.revision.next())
                .unwrap_or(Revision::INITIAL),
            created_at: existing.map(|current| current.created_at).unwrap_or(now),
            updated_at: now,
        };
        let id = exception.id;
        let outcome = self
            .opportunities
            .upsert(&exception, write_expected)
            .await?;
        commit_outcome(OPPORTUNITY, id, write_expected, outcome)
    }

    pub async fn requirements(&self, focus: Option<Date>) -> Result<ShoppingList> {
        let today = self.clock.now().date();
        let cadence = self.cadence.get().await?;
        let lookahead = today + Duration::days(OPPORTUNITY_LOOKAHEAD_DAYS);
        let exceptions = self.opportunities.list_in_range(today, lookahead).await?;
        let opportunities = expand_opportunities(cadence.as_ref(), &exceptions, today, lookahead);

        let focus = focus.or_else(|| opportunities.first().map(|first| first.date));

        let window_end = today + Duration::days(PLANNING_WINDOW_DAYS);

        let snapshot = self.stock.snapshot(today, window_end).await?;
        let open_purchases: Vec<Purchase> = self
            .purchases
            .list_open()
            .await?
            .into_iter()
            .filter(|purchase| match purchase.opportunity_date {
                Some(date) => Some(date) == focus,
                None => purchase.state == PurchaseState::Pending,
            })
            .collect();

        let mut requirements = Vec::new();

        for row in &snapshot.report.ingredients {
            let pool = snapshot
                .pools
                .get(&row.ingredient_id)
                .cloned()
                .unwrap_or_default();
            let items = items_for(&snapshot.items, &pool);
            let claims = claims_for_pool(
                &snapshot.report.claims,
                DemandSubject::ingredient(row.ingredient_id),
                &pool,
            );
            let section = self.section_for_pool(row.ingredient_id, &pool).await?;

            requirements.extend(build(
                DemandSubject::ingredient(row.ingredient_id),
                row.name.clone(),
                &row.availability,
                &items,
                &claims,
                section,
                &pool,
                &opportunities,
                &open_purchases,
                &row.demand_gaps,
            ));
        }

        for row in &snapshot.report.prepared_meals {
            let pool = snapshot
                .prepared_meal_pools
                .get(&row.prepared_meal_id)
                .cloned()
                .unwrap_or_default();
            let items = items_for(&snapshot.items, &pool);
            let claims = claims_for_pool(
                &snapshot.report.claims,
                DemandSubject::prepared_meal(row.prepared_meal_id),
                &pool,
            );
            let section = self
                .section_for_prepared_meal_pool(row.prepared_meal_id, &pool)
                .await?;

            requirements.extend(build(
                DemandSubject::prepared_meal(row.prepared_meal_id),
                row.name.clone(),
                &row.availability,
                &items,
                &claims,
                section,
                &pool,
                &opportunities,
                &open_purchases,
                &row.demand_gaps,
            ));
        }

        let unpooled: Vec<ProductId> = snapshot
            .report
            .products
            .iter()
            .map(|row| row.product_id)
            .filter(|product_id| {
                !snapshot
                    .pools
                    .values()
                    .any(|pool| pool.contains(product_id))
                    && !snapshot
                        .prepared_meal_pools
                        .values()
                        .any(|pool| pool.contains(product_id))
            })
            .collect();
        let products = self.products.get_many(&unpooled).await?;
        let by_id: HashMap<ProductId, &crate::domain::Product> = products
            .iter()
            .map(|product| (product.id, product))
            .collect();

        for row in &snapshot.report.products {
            let Some(product) = by_id.get(&row.product_id) else {
                continue;
            };
            let pool = vec![row.product_id];
            let items = items_for(&snapshot.items, &pool);
            let claims: Vec<DemandClaim> = snapshot
                .report
                .claims
                .iter()
                .filter(|claim| claim.subject == DemandSubject::product(row.product_id))
                .cloned()
                .collect();
            let section = product.shopping_section.unwrap_or(ShoppingSection::Other);

            requirements.extend(build(
                DemandSubject::product(row.product_id),
                product.name.clone(),
                &row.availability,
                &items,
                &claims,
                section,
                &pool,
                &opportunities,
                &open_purchases,
                &row.demand_gaps,
            ));
        }

        let order = self.settings.get().await?.section_order;
        requirements.sort_by(|a, b| {
            order
                .rank(a.section)
                .cmp(&order.rank(b.section))
                .then_with(|| a.name.cmp(&b.name))
        });

        let all_manual = self.list_items.list().await?;
        let mut counts: Vec<ShopCount> = opportunities
            .iter()
            .map(|opportunity| {
                let date = opportunity.date;
                let derived = requirements
                    .iter()
                    .filter(|requirement| {
                        matches!(requirement.assignment, Assignment::Opportunity { date: on } if on == date)
                    })
                    .count();
                let by_hand = all_manual
                    .iter()
                    .filter(|item| item.opportunity_date == Some(date))
                    .count();
                ShopCount {
                    date,
                    items: derived + by_hand,
                }
            })
            .collect();
        if let Some(first) = counts.first_mut() {
            first.items += requirements
                .iter()
                .filter(|requirement| {
                    matches!(requirement.assignment, Assignment::NeedsEarlierOpportunity)
                })
                .count()
                + all_manual
                    .iter()
                    .filter(|item| item.opportunity_date.is_none())
                    .count();
        }

        if let Some(focus) = focus {
            requirements.retain(|requirement| {
                matches!(requirement.assignment, Assignment::Opportunity { date } if date == focus)
                    || matches!(requirement.assignment, Assignment::NeedsEarlierOpportunity)
            });
        }

        let mut manual = all_manual;
        manual.retain(|item| match (item.opportunity_date, focus) {
            (Some(on), Some(focus)) => on == focus,
            (Some(_), None) => false,
            (None, _) => true,
        });
        manual.sort_by(|a, b| {
            order
                .rank(a.section.unwrap_or(ShoppingSection::Other))
                .cmp(&order.rank(b.section.unwrap_or(ShoppingSection::Other)))
                .then_with(|| a.name.cmp(&b.name))
        });

        let claimed: Vec<PurchaseId> = requirements
            .iter()
            .flat_map(|requirement| requirement.purchases.iter())
            .map(|purchase| purchase.id)
            .collect();
        let unplanned: Vec<Purchase> = open_purchases
            .into_iter()
            .filter(|purchase| purchase.state == PurchaseState::Pending)
            .filter(|purchase| !claimed.contains(&purchase.id))
            .collect();

        let trip = match focus {
            Some(focus) => self.trips.for_date(focus).await?,
            None => None,
        };

        Ok(ShoppingList {
            opportunities,
            focus,
            requirements,
            manual,
            unplanned,
            trip,
            counts,
            cadence_configured: cadence.is_some(),
        })
    }

    async fn section_for_pool(
        &self,
        ingredient_id: IngredientId,
        pool: &[ProductId],
    ) -> Result<ShoppingSection> {
        if let Some(ingredient) = self.ingredients.get(ingredient_id).await?
            && let Some(section) = ingredient.shopping_section
        {
            return Ok(section);
        }
        for product in self.products.get_many(pool).await? {
            if let Some(section) = product.shopping_section {
                return Ok(section);
            }
        }
        Ok(ShoppingSection::Other)
    }

    async fn section_for_prepared_meal_pool(
        &self,
        prepared_meal_id: PreparedMealId,
        pool: &[ProductId],
    ) -> Result<ShoppingSection> {
        if let Some(prepared_meal) = self.prepared_meals.get(prepared_meal_id).await?
            && let Some(section) = prepared_meal.shopping_section
        {
            return Ok(section);
        }
        for product in self.products.get_many(pool).await? {
            if let Some(section) = product.shopping_section {
                return Ok(section);
            }
        }
        Ok(ShoppingSection::Other)
    }

    pub async fn purchases(&self, query: &PurchaseQuery) -> Result<Paginated<Purchase>> {
        self.purchases.list(query).await
    }

    pub async fn pending_purchases(&self) -> Result<Vec<Purchase>> {
        Ok(self
            .purchases
            .list_open()
            .await?
            .into_iter()
            .filter(|purchase| purchase.state == PurchaseState::Pending)
            .collect())
    }

    pub async fn awaiting_put_away(&self) -> Result<Vec<Purchase>> {
        let today = self.clock.now().date();
        let pending = self.pending_purchases().await?;

        let mut waiting = Vec::new();
        for purchase in pending {
            let settled = match purchase.opportunity_date {
                None => true,
                Some(date) if date < today => true,
                Some(date) => self
                    .trips
                    .for_date(date)
                    .await?
                    .is_some_and(|trip| trip.is_finished()),
            };
            if settled {
                waiting.push(purchase);
            }
        }
        Ok(waiting)
    }

    pub async fn put_away(
        &self,
        id: PurchaseId,
        expected: Revision,
        product_id: ProductId,
        quantity: Quantity,
        storage: Option<StorageLocation>,
        actor: UserId,
    ) -> Result<Purchase> {
        let Some(mut purchase) = self.purchases.get(id).await? else {
            return Err(CoreError::not_found(PURCHASE, id.to_string()));
        };
        if purchase.state == PurchaseState::Reconciled {
            return Ok(purchase);
        }
        if purchase.state == PurchaseState::Cancelled {
            return Err(CoreError::conflict("That purchase was cancelled."));
        }

        purchase.product_id = Some(product_id);
        purchase.quantity = Some(quantity);

        let Some(stock) = self.stock_for(&purchase, actor, storage).await? else {
            return Err(CoreError::conflict("Say which product and how much."));
        };

        purchase.state = PurchaseState::Reconciled;
        purchase.stock_item_id = Some(stock.item.id);
        purchase.updated_at = self.clock.now();
        purchase.revision = expected.next();

        let outcome = self
            .purchases
            .update(&purchase, expected, Some(&stock))
            .await?;
        commit_outcome(PURCHASE, id, expected, outcome)?;
        Ok(purchase)
    }

    pub async fn record_purchase(&self, input: NewPurchase, actor: UserId) -> Result<Purchase> {
        input.validate()?;
        let now = self.clock.now();
        let purchase = Purchase {
            id: PurchaseId::new(),
            ingredient_id: input.ingredient_id,
            prepared_meal_id: input.prepared_meal_id,
            product_id: input.product_id,
            name: input
                .name
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty()),
            quantity: input.quantity,
            opportunity_date: input.opportunity_date,
            state: PurchaseState::Pending,
            stock_item_id: None,
            purchased_at: now,
            actor_user_id: actor,
            note: input.note,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };

        self.purchases.insert(&purchase, None).await?;
        Ok(purchase)
    }

    pub async fn update_purchase(
        &self,
        id: PurchaseId,
        expected: Revision,
        patch: PurchasePatch,
    ) -> Result<Purchase> {
        patch.validate()?;
        let Some(mut purchase) = self.purchases.get(id).await? else {
            return Err(CoreError::not_found(PURCHASE, id.to_string()));
        };
        if purchase.state == PurchaseState::Reconciled {
            return Err(CoreError::conflict(
                "That purchase is already in stock. Correct the stock item instead.",
            ));
        }

        if let Some(product_id) = patch.product_id {
            purchase.product_id = Some(product_id);
        }
        if let Some(quantity) = patch.quantity {
            purchase.quantity = Some(quantity);
        }
        purchase.note = patch.note.apply(purchase.note);
        if patch.cancelled == Some(true) {
            purchase.state = PurchaseState::Cancelled;
        }

        purchase.updated_at = self.clock.now();
        purchase.revision = expected.next();

        let outcome = self.purchases.update(&purchase, expected, None).await?;
        commit_outcome(PURCHASE, id, expected, outcome)?;
        Ok(purchase)
    }

    pub async fn finish_shop(
        &self,
        date: Date,
        actor: UserId,
        expected_trip: Revision,
    ) -> Result<FinishedShop> {
        let trip = self.trips.for_date(date).await?;
        let actual_trip = trip
            .as_ref()
            .map(|trip| trip.revision)
            .unwrap_or(Revision::UNRECORDED);
        if actual_trip != expected_trip {
            return Err(CoreError::RevisionMismatch {
                resource: "shopping trip",
                id: date.to_string(),
                expected: expected_trip,
                actual: actual_trip,
            });
        }

        let pending: Vec<Purchase> = self
            .purchases
            .list_open()
            .await?
            .into_iter()
            .filter(|purchase| {
                purchase.state == PurchaseState::Pending && purchase.opportunity_date == Some(date)
            })
            .collect();

        let mut finished = FinishedShop::default();
        let mut ready: Vec<FinishedPurchase> = Vec::new();
        for mut purchase in pending {
            let Some(stock) = self.stock_for(&purchase, actor, None).await? else {
                finished.still_pending += 1;
                continue;
            };
            let expected = purchase.revision;
            purchase.state = PurchaseState::Reconciled;
            purchase.stock_item_id = Some(stock.item.id);
            purchase.updated_at = self.clock.now();
            purchase.revision = expected.next();
            ready.push(FinishedPurchase {
                purchase,
                expected,
                stock,
            });
        }

        if !ready.is_empty() {
            match self.purchases.finish(&ready).await? {
                UpdateOutcome::Updated => finished.stocked = ready.len(),
                UpdateOutcome::NotFound => {
                    let id = ready[0].purchase.id;
                    return Err(CoreError::not_found(PURCHASE, id.to_string()));
                }
                UpdateOutcome::RevisionMismatch { actual } => {
                    let held = &ready[0];
                    return Err(CoreError::RevisionMismatch {
                        resource: PURCHASE,
                        id: held.purchase.id.to_string(),
                        expected: held.expected,
                        actual,
                    });
                }
            }
        }

        self.list_items.delete_for_opportunity(date).await?;

        if let Some(mut trip) = trip
            && !trip.is_finished()
        {
            let expected = trip.revision;
            trip.state = TripState::Finished;
            trip.finished_at = Some(self.clock.now());
            trip.updated_at = self.clock.now();
            trip.revision = expected.next();
            self.trips.update(&trip, expected).await?;
        }

        Ok(finished)
    }

    async fn stock_for(
        &self,
        purchase: &Purchase,
        actor: UserId,
        storage: Option<StorageLocation>,
    ) -> Result<Option<NewStockFromPurchase>> {
        let (Some(product_id), Some(quantity)) = (purchase.product_id, purchase.quantity) else {
            return Ok(None);
        };
        let Some(product) = self.products.get(product_id).await? else {
            return Err(CoreError::not_found("product", product_id.to_string()));
        };
        if product.is_archived() {
            return Err(CoreError::conflict("That product is archived."));
        }

        let section = match product.shopping_section {
            Some(section) => Some(section),
            None => match product.mapped_ingredient_id {
                Some(ingredient_id) => self
                    .ingredients
                    .get(ingredient_id)
                    .await?
                    .and_then(|ingredient| ingredient.shopping_section),
                None => None,
            },
        };

        let input = NewStockItem {
            subject: StockSubject::product(product_id),
            level: StockLevel::Exact { quantity },
            storage_location: storage.unwrap_or_else(|| storage_for(section)),
            source_date: None,
            usability_deadline: None,
            note: purchase.note.clone(),
        };
        input.validate()?;

        let now = self.clock.now();
        let item = StockItem {
            id: crate::domain::StockItemId::new(),
            subject: input.subject,
            level: input.level,
            storage_location: input.storage_location,
            source_date: None,
            usability_deadline: None,
            note: input.note,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        let event = NewStockEvent {
            kind: StockEventKind::Added,
            quantity_delta: Some(quantity),
            actor_user_id: Some(actor),
            subject_member_id: None,
            source: Some(StockEventSource {
                kind: StockEffectSource::Purchase,
                id: purchase.id.as_uuid(),
                label: "Purchase".to_owned(),
            }),
            reverses_event_id: None,
            note: None,
        };
        Ok(Some(NewStockFromPurchase { item, event }))
    }
}

fn items_for(items: &[StockItem], pool: &[ProductId]) -> Vec<StockItem> {
    items
        .iter()
        .filter(|item| item.product_id().is_some_and(|id| pool.contains(&id)))
        .cloned()
        .collect()
}

fn claims_for_pool(
    claims: &[DemandClaim],
    subject: DemandSubject,
    pool: &[ProductId],
) -> Vec<DemandClaim> {
    claims
        .iter()
        .filter(|claim| match claim.subject {
            DemandSubject::Ingredient { .. } | DemandSubject::PreparedMeal { .. } => {
                claim.subject == subject
            }
            DemandSubject::Product { product_id } => pool.contains(&product_id),
            DemandSubject::PreparedPortion { .. } | DemandSubject::CookedFood { .. } => false,
        })
        .cloned()
        .collect()
}

fn storage_for(section: Option<ShoppingSection>) -> StorageLocation {
    match section {
        Some(ShoppingSection::Frozen) => StorageLocation::Frozen,
        Some(
            ShoppingSection::FreshProduce | ShoppingSection::MeatFish | ShoppingSection::Dairy,
        ) => StorageLocation::Chilled,
        _ => StorageLocation::Ambient,
    }
}

fn assignment_order(assignment: &Assignment) -> (u8, Option<Date>) {
    match assignment {
        Assignment::NeedsEarlierOpportunity => (0, None),
        Assignment::Opportunity { date } => (1, Some(*date)),
        Assignment::Unassigned => (2, None),
    }
}

fn bucket_for_purchase(purchase: &Purchase, assignments: &[Assignment]) -> Option<usize> {
    if let Some(date) = purchase.opportunity_date
        && let Some(index) = assignments.iter().position(
            |assignment| matches!(assignment, Assignment::Opportunity { date: on } if *on == date),
        )
    {
        return Some(index);
    }
    (!assignments.is_empty()).then_some(0)
}

fn bucket_quantity(held: &[UncoveredClaim], gaps: &mut Vec<DemandGap>) -> Option<Quantity> {
    let mut running: Option<Quantity> = None;
    for uncovered in held {
        match running {
            None => running = Some(uncovered.missing),
            Some(total) => match uncovered.missing.convert_to(total.unit) {
                Ok(converted) => {
                    running = Some(Quantity::new(total.amount + converted.amount, total.unit));
                }
                Err(_) => {
                    if !gaps.contains(&DemandGap::IncompatibleUnits) {
                        gaps.push(DemandGap::IncompatibleUnits);
                    }
                }
            },
        }
    }
    running
}

#[allow(clippy::too_many_arguments)]
fn build(
    subject: DemandSubject,
    name: String,
    availability: &Availability,
    items: &[StockItem],
    claims: &[DemandClaim],
    section: ShoppingSection,
    pool: &[ProductId],
    opportunities: &[ShoppingOpportunity],
    open_purchases: &[Purchase],
    row_gaps: &[DemandGap],
) -> Vec<ShoppingRequirement> {
    if matches!(availability, Availability::AssumedAvailable) {
        return Vec::new();
    }

    let coverage = cover(items, claims);
    if coverage.uncovered.is_empty() {
        return Vec::new();
    }

    let certainty = match availability {
        _ if row_gaps.contains(&DemandGap::FoodHasNoProducts) => Certainty::Suggested {
            reason: SuggestionReason::NoProductYet,
        },
        Availability::Unknown => Certainty::Suggested {
            reason: SuggestionReason::UnknownAvailability,
        },
        _ if coverage.assumption_only => Certainty::Suggested {
            reason: SuggestionReason::AssumptionOnly,
        },
        _ => Certainty::Definite,
    };

    let mut buckets: Vec<(Assignment, Vec<UncoveredClaim>)> = Vec::new();
    for uncovered in coverage.uncovered {
        let assignment = assign(Some(uncovered.claim.planned_on), opportunities);
        match buckets
            .iter_mut()
            .find(|(existing, _)| *existing == assignment)
        {
            Some((_, held)) => held.push(uncovered),
            None => buckets.push((assignment, vec![uncovered])),
        }
    }
    buckets.sort_by_key(|(assignment, _)| assignment_order(assignment));

    let assignments: Vec<Assignment> = buckets.iter().map(|(assignment, _)| *assignment).collect();
    let matching: Vec<&Purchase> = open_purchases
        .iter()
        .filter(|purchase| purchase.matches(&subject, pool))
        .collect();

    buckets
        .into_iter()
        .enumerate()
        .map(|(index, (assignment, held))| {
            let mut gaps = coverage.gaps.clone();
            for gap in row_gaps {
                if !gaps.contains(gap) {
                    gaps.push(*gap);
                }
            }
            let quantity = bucket_quantity(&held, &mut gaps);
            let required_by = held.iter().map(|held| held.claim.planned_on).min();
            let use_by_at_least = held.iter().map(|held| held.claim.planned_on).max();
            let purchases: Vec<Purchase> = matching
                .iter()
                .filter(|purchase| bucket_for_purchase(purchase, &assignments) == Some(index))
                .map(|purchase| (*purchase).clone())
                .collect();

            ShoppingRequirement {
                subject,
                name: name.clone(),
                quantity,
                required_by,
                use_by_at_least,
                section,
                certainty,
                assignment,
                claims: held.into_iter().map(|held| held.claim).collect(),
                gaps,
                purchases,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "shopping_tests.rs"]
mod tests;
