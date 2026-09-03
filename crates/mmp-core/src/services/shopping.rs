use std::collections::HashMap;
use std::sync::Arc;

use time::{Date, Duration};

use crate::domain::{
    Assignment, Availability, Certainty, DemandClaim, DemandSubject, ExceptionState, IngredientId,
    NewPurchase, NewShoppingCadence, NewStockEvent, NewStockItem, OpportunityException, ProductId,
    Purchase, PurchaseId, PurchasePatch, PurchaseState, Revision, ShoppingCadence,
    ShoppingOpportunity, ShoppingOpportunityId, ShoppingRequirement, ShoppingSection,
    StockEffectSource, StockEventKind, StockEventSource, StockItem, StockLevel, StorageLocation,
    SuggestionReason, UserId, assign, cover, expand_opportunities,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, IngredientRepository, NewStockFromPurchase, Paginated, ProductRepository, PurchaseQuery,
    PurchaseRepository, ShoppingCadenceRepository, ShoppingOpportunityRepository, UpdateOutcome,
};
use crate::services::StockService;

const CADENCE: &str = "shopping cadence";
const OPPORTUNITY: &str = "shopping opportunity";
const PURCHASE: &str = "purchase";

const OPPORTUNITY_LOOKAHEAD_DAYS: i64 = 70;

const UNPLANNED_QUERY_DAYS: i64 = 28;

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
    pub cadence_configured: bool,
}

#[derive(Clone)]
pub struct ShoppingService {
    cadence: Arc<dyn ShoppingCadenceRepository>,
    opportunities: Arc<dyn ShoppingOpportunityRepository>,
    purchases: Arc<dyn PurchaseRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    products: Arc<dyn ProductRepository>,
    stock: StockService,
    clock: Arc<dyn Clock>,
}

impl ShoppingService {
    pub fn new(
        cadence: Arc<dyn ShoppingCadenceRepository>,
        opportunities: Arc<dyn ShoppingOpportunityRepository>,
        purchases: Arc<dyn PurchaseRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        products: Arc<dyn ProductRepository>,
        stock: StockService,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            cadence,
            opportunities,
            purchases,
            ingredients,
            products,
            stock,
            clock,
        }
    }

    pub async fn cadence(&self) -> Result<Option<ShoppingCadence>> {
        self.cadence.get().await
    }

    pub async fn set_cadence(&self, input: NewShoppingCadence) -> Result<ShoppingCadence> {
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
        self.cadence.set(&cadence).await?;
        Ok(cadence)
    }

    pub async fn clear_cadence(&self) -> Result<()> {
        if self.cadence.get().await?.is_none() {
            return Err(CoreError::not_found(CADENCE, "singleton"));
        }
        self.cadence.clear().await
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

    pub async fn move_opportunity(&self, occurrence: Date, to: Date) -> Result<()> {
        self.record_exception(ExceptionState::Moved, Some(occurrence), Some(to), None)
            .await
    }

    pub async fn skip_opportunity(&self, occurrence: Date) -> Result<()> {
        self.record_exception(ExceptionState::Skipped, Some(occurrence), None, None)
            .await
    }

    pub async fn add_one_off(&self, date: Date, note: Option<String>) -> Result<()> {
        self.record_exception(ExceptionState::OneOff, None, Some(date), note)
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
    ) -> Result<()> {
        let now = self.clock.now();
        let existing = match generated_for {
            Some(occurrence) => self.opportunities.find_for_occurrence(occurrence).await?,
            None => None,
        };
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
        self.opportunities.upsert(&exception).await
    }

    pub async fn requirements(&self, focus: Option<Date>) -> Result<ShoppingList> {
        let today = self.clock.now().date();
        let cadence = self.cadence.get().await?;
        let lookahead = today + Duration::days(OPPORTUNITY_LOOKAHEAD_DAYS);
        let exceptions = self.opportunities.list_in_range(today, lookahead).await?;
        let opportunities = expand_opportunities(cadence.as_ref(), &exceptions, today, lookahead);

        let focus = focus.or_else(|| opportunities.first().map(|first| first.date));

        let window_end = match opportunities.as_slice() {
            [] => today + Duration::days(UNPLANNED_QUERY_DAYS),
            [only] => only.date + Duration::days(14),
            [_, second, ..] => second.date,
        };

        let snapshot = self.stock.shopping_snapshot(today, window_end).await?;
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
            let claims = claims_for_pool(&snapshot.report.claims, row.ingredient_id, &pool);
            let section = self.section_for_pool(row.ingredient_id, &pool).await?;

            if let Some(requirement) = build(
                DemandSubject::ingredient(row.ingredient_id),
                row.name.clone(),
                &row.availability,
                &items,
                &claims,
                section,
                &pool,
                &opportunities,
                &open_purchases,
            ) {
                requirements.push(requirement);
            }
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
            let section = product
                .shopping_section
                .as_deref()
                .and_then(|section| section.parse().ok())
                .unwrap_or(ShoppingSection::Other);

            if let Some(requirement) = build(
                DemandSubject::product(row.product_id),
                product.name.clone(),
                &row.availability,
                &items,
                &claims,
                section,
                &pool,
                &opportunities,
                &open_purchases,
            ) {
                requirements.push(requirement);
            }
        }

        requirements.sort_by(|a, b| {
            a.section
                .order()
                .cmp(&b.section.order())
                .then_with(|| a.name.cmp(&b.name))
        });

        if let Some(focus) = focus {
            requirements.retain(|requirement| {
                matches!(requirement.assignment, Assignment::Opportunity { date } if date == focus)
                    || matches!(requirement.assignment, Assignment::NeedsEarlierOpportunity)
            });
        }

        Ok(ShoppingList {
            opportunities,
            focus,
            requirements,
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
            if let Some(section) = product
                .shopping_section
                .as_deref()
                .and_then(|section| section.parse::<ShoppingSection>().ok())
            {
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

    pub async fn record_purchase(&self, input: NewPurchase, actor: UserId) -> Result<Purchase> {
        input.validate()?;
        let now = self.clock.now();
        let purchase = Purchase {
            id: PurchaseId::new(),
            ingredient_id: input.ingredient_id,
            product_id: input.product_id,
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

        match self.purchases.update(&purchase, expected, None).await? {
            UpdateOutcome::Updated => Ok(purchase),
            UpdateOutcome::NotFound => Err(CoreError::not_found(PURCHASE, id.to_string())),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: PURCHASE,
                id: id.to_string(),
                expected,
                actual,
            }),
        }
    }

    pub async fn finish_shop(&self, date: Date, actor: UserId) -> Result<FinishedShop> {
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
        for mut purchase in pending {
            let Some(stock) = self.stock_for(&purchase, actor).await? else {
                finished.still_pending += 1;
                continue;
            };
            let expected = purchase.revision;
            purchase.state = PurchaseState::Reconciled;
            purchase.stock_item_id = Some(stock.item.id);
            purchase.updated_at = self.clock.now();
            purchase.revision = expected.next();

            match self
                .purchases
                .update(&purchase, expected, Some(&stock))
                .await?
            {
                UpdateOutcome::Updated => finished.stocked += 1,
                UpdateOutcome::NotFound => {
                    return Err(CoreError::not_found(PURCHASE, purchase.id.to_string()));
                }
                UpdateOutcome::RevisionMismatch { actual } => {
                    return Err(CoreError::RevisionMismatch {
                        resource: PURCHASE,
                        id: purchase.id.to_string(),
                        expected,
                        actual,
                    });
                }
            }
        }

        Ok(finished)
    }

    async fn stock_for(
        &self,
        purchase: &Purchase,
        actor: UserId,
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

        let input = NewStockItem {
            product_id,
            level: StockLevel::Exact { quantity },
            storage_location: StorageLocation::Ambient,
            source_date: None,
            usability_deadline: None,
            note: purchase.note.clone(),
        };
        input.validate()?;

        let now = self.clock.now();
        let item = StockItem {
            id: crate::domain::StockItemId::new(),
            product_id,
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
        .filter(|item| pool.contains(&item.product_id))
        .cloned()
        .collect()
}

fn claims_for_pool(
    claims: &[DemandClaim],
    ingredient_id: IngredientId,
    pool: &[ProductId],
) -> Vec<DemandClaim> {
    claims
        .iter()
        .filter(|claim| match claim.subject {
            DemandSubject::Ingredient { ingredient_id: id } => id == ingredient_id,
            DemandSubject::Product { product_id } => pool.contains(&product_id),
        })
        .cloned()
        .collect()
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
) -> Option<ShoppingRequirement> {
    if matches!(availability, Availability::AssumedAvailable) {
        return None;
    }

    let coverage = cover(items, claims);
    let shortfall = coverage.shortfall?;

    let certainty = match availability {
        Availability::Unknown => Certainty::Suggested {
            reason: SuggestionReason::UnknownAvailability,
        },
        _ if coverage.assumption_only => Certainty::Suggested {
            reason: SuggestionReason::AssumptionOnly,
        },
        _ => Certainty::Definite,
    };

    let purchases: Vec<Purchase> = open_purchases
        .iter()
        .filter(|purchase| purchase.matches(&subject, pool))
        .cloned()
        .collect();

    Some(ShoppingRequirement {
        subject,
        name,
        quantity: Some(shortfall),
        required_by: coverage.required_by,
        use_by_at_least: coverage.use_by_at_least,
        section,
        certainty,
        assignment: assign(coverage.required_by, opportunities),
        claims: coverage.uncovered,
        gaps: coverage.gaps,
        purchases,
    })
}

#[cfg(test)]
#[path = "shopping_tests.rs"]
mod tests;
