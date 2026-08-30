use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use time::{Date, Duration, OffsetDateTime, Time};

use rust_decimal::Decimal;

use crate::domain::{
    AllocationOutcome, ComponentPreparation, ConfirmMealPlanComponent, ConfirmMealPlanEntry,
    ConsumedAmount, ConsumedNutrition, ConsumptionRecord, ConsumptionRecordId, HouseholdMemberId,
    MealItemRef, MealParticipant, MealParticipantAllocation, MealParticipantAllocationId,
    MealParticipantId, MealPlanComponent, MealPlanComponentId, MealPlanComponentSnapshot,
    MealPlanEntry, MealPlanEntryId, MealPlanEntryPatch, MealPlanScope, MealPlanStatus, MealSlot,
    NUTRIENT_KEYS, NewMealParticipant, NewMealPlanComponent, NewMealPlanEntry, NutritionFacts,
    NutritionGoals, NutritionQuality, OutcomeActor, ParticipantStatus, Product, ProductId, Recipe,
    RecipeId, RecipeRequirement, Revision, SetMealParticipants, UserId, derive_component_status,
    derive_entry_status, derive_participant_status, nutrition_for, preparation_for,
    recipe_nutrition, recipe_nutrition_for, resolve_on, sum_nutrition, validate_components,
    validate_participants,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, ConsumptionRecordRepository, HouseholdMemberRepository, HouseholdSettingsRepository,
    MealPlanComponentUpdate, MealPlanQuery, MealPlanRepository, MemberQuery,
    NutritionTargetRepository, PageRequest, ProductRepository, RecipeRepository, SnapshotOp,
    UpdateOutcome,
};

use super::fulfilment::RecipeFulfilments;

const MEAL_PLAN_ENTRY: &str = "meal plan entry";
const MEAL_PLAN_COMPONENT: &str = "meal plan component";
const PRODUCT: &str = "product";
const RECIPE: &str = "recipe";

#[derive(Debug, Clone, Default)]
pub struct NutritionSummary {
    pub nutrition: NutritionFacts,
    pub unknown_count: i64,
    pub partial_count: i64,
}

#[derive(Debug, Clone)]
pub struct MealPlanComponentView {
    pub component: MealPlanComponent,
    pub item_name: String,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
    pub consumption_record: Option<ConsumptionRecord>,
    pub preparation: ComponentPreparation,
    pub subject_status: MealPlanStatus,
}

#[derive(Debug, Clone)]
pub struct MealParticipantView {
    pub member_id: HouseholdMemberId,
    pub display_name: String,
    pub status: MealPlanStatus,
    pub allocations: Vec<MealParticipantAllocation>,
    pub nutrition: NutritionSummary,
}

#[derive(Debug, Clone)]
pub struct MealPlanEntryView {
    pub entry: MealPlanEntry,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub components: Vec<MealPlanComponentView>,
    pub participants: Vec<MealParticipantView>,
    pub planned: NutritionSummary,
    pub actual: Option<NutritionSummary>,
    pub needs_attention: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MealItemSource {
    Planned {
        entry_id: MealPlanEntryId,
        component_id: MealPlanComponentId,
    },
    Logged {
        record_id: ConsumptionRecordId,
    },
}

#[derive(Debug, Clone)]
pub struct MealItem {
    pub source: MealItemSource,
    pub record_id: Option<ConsumptionRecordId>,
    pub status: MealPlanStatus,
    pub item: MealItemRef,
    pub item_name: String,
    pub amount: ConsumedAmount,
    pub planned_amount: Option<ConsumedAmount>,
    pub planned_on: Option<Date>,
    pub at: Option<Time>,
    pub consumed_at: Option<time::OffsetDateTime>,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
    pub needs_attention: bool,
    pub revision: Revision,
    pub record_revision: Option<Revision>,
}

type MealItemOrder = uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MealSlotView {
    pub slot: MealSlot,
    pub items: Vec<MealItem>,
    pub nutrition: NutritionSummary,
}

#[derive(Debug, Clone)]
pub struct MealPlanDay {
    pub date: Date,
    pub entries: Vec<MealPlanEntryView>,
    pub slots: Vec<MealSlotView>,
    pub actual: NutritionSummary,
    pub remaining_planned: NutritionSummary,
    pub projected: NutritionSummary,
    pub target: Option<NutritionGoals>,
}

#[derive(Debug, Clone)]
pub struct MealPlanWeek {
    pub member_id: crate::domain::HouseholdMemberId,
    pub week_start: Date,
    pub week_end: Date,
    pub days: Vec<MealPlanDay>,
    pub actual: NutritionSummary,
    pub remaining_planned: NutritionSummary,
    pub projected: NutritionSummary,
    pub target: Option<NutritionGoals>,
    pub insufficient_target_coverage: Vec<String>,
}

struct RecipeCard {
    name: String,
    per_serving: ConsumedNutrition,
    archived: bool,
}

struct ResolvedItem {
    name: String,
    nutrition: ConsumedNutrition,
    resolvable: bool,
}

struct ItemCatalogue {
    products: HashMap<ProductId, Product>,
    recipes: HashMap<RecipeId, RecipeCard>,
}

impl ItemCatalogue {
    fn name_of(&self, item: MealItemRef) -> String {
        match item {
            MealItemRef::Product { product_id } => self
                .products
                .get(&product_id)
                .map(|product| product.name.clone())
                .unwrap_or_else(|| "Missing product".to_owned()),
            MealItemRef::Recipe { recipe_id } => self
                .recipes
                .get(&recipe_id)
                .map(|card| card.name.clone())
                .unwrap_or_else(|| "Missing recipe".to_owned()),
        }
    }

    fn resolve(&self, item: MealItemRef, amount: &ConsumedAmount) -> ResolvedItem {
        match item {
            MealItemRef::Product { product_id } => match self.products.get(&product_id) {
                Some(product) => ResolvedItem {
                    name: product.name.clone(),
                    nutrition: nutrition_for(product, amount),
                    resolvable: amount.resolve(product).is_ok(),
                },
                None => ResolvedItem {
                    name: "Missing product".to_owned(),
                    nutrition: ConsumedNutrition::unknown(),
                    resolvable: false,
                },
            },
            MealItemRef::Recipe { recipe_id } => match self.recipes.get(&recipe_id) {
                Some(card) => ResolvedItem {
                    name: card.name.clone(),
                    nutrition: recipe_nutrition_for(&card.per_serving, amount),
                    resolvable: matches!(amount, ConsumedAmount::Servings(_)) && !card.archived,
                },
                None => ResolvedItem {
                    name: "Missing recipe".to_owned(),
                    nutrition: ConsumedNutrition::unknown(),
                    resolvable: false,
                },
            },
        }
    }
}

fn recipe_card(recipe: &Recipe, fulfilments: &RecipeFulfilments) -> RecipeCard {
    let per_serving = recipe_nutrition(
        recipe
            .components
            .iter()
            .map(|component| (&component.amount, fulfilments.get(&component.requirement))),
        recipe.servings,
    );
    RecipeCard {
        name: recipe.name.clone(),
        per_serving,
        archived: recipe.is_archived(),
    }
}

#[derive(Clone)]
pub struct MealPlanService {
    plans: Arc<dyn MealPlanRepository>,
    products: Arc<dyn ProductRepository>,
    recipes: Arc<dyn RecipeRepository>,
    consumption: Arc<dyn ConsumptionRecordRepository>,
    targets: Arc<dyn NutritionTargetRepository>,
    members: Arc<dyn HouseholdMemberRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl MealPlanService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plans: Arc<dyn MealPlanRepository>,
        products: Arc<dyn ProductRepository>,
        recipes: Arc<dyn RecipeRepository>,
        consumption: Arc<dyn ConsumptionRecordRepository>,
        targets: Arc<dyn NutritionTargetRepository>,
        members: Arc<dyn HouseholdMemberRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            plans,
            products,
            recipes,
            consumption,
            targets,
            members,
            settings,
            clock,
        }
    }

    pub async fn create(&self, input: NewMealPlanEntry) -> Result<MealPlanEntryView> {
        ensure_not_past(&*self.clock, input.planned_on)?;
        self.create_unchecked(input).await
    }

    pub async fn create_unchecked(&self, input: NewMealPlanEntry) -> Result<MealPlanEntryView> {
        validate_components(&input.components)?;
        if input.scope == MealPlanScope::Member && input.member_id.is_none() {
            return Err(CoreError::conflict(
                "A personal meal needs a household member.",
            ));
        }
        self.validate_component_items(&input.components, &HashSet::new(), input.actor_id)
            .await?;

        let now = self.clock.now();
        let components = make_components(input.components);

        let (owner_member_id, extra_member_ids) = match input.scope {
            MealPlanScope::Member => (input.member_id, Vec::new()),
            MealPlanScope::Household => {
                let members = if self.settings.get().await?.default_all_members_participate {
                    self.active_member_ids().await?
                } else {
                    Vec::new()
                };
                (None, members)
            }
        };

        let mut participants = Vec::new();
        if let Some(member_id) = owner_member_id {
            participants.push(build_participant(member_id, &components, now, true));
        }
        for member_id in extra_member_ids {
            participants.push(build_participant(member_id, &components, now, false));
        }
        for member_id in participants.iter().map(|p| p.member_id).collect::<Vec<_>>() {
            self.ensure_slot_free(member_id, input.planned_on, input.slot, None)
                .await?;
        }

        let mut entry = MealPlanEntry {
            id: input.id.unwrap_or_default(),
            scope: input.scope,
            member_id: input.member_id,
            planned_on: input.planned_on,
            planned_time: input
                .slot
                .allows_planned_time()
                .then_some(input.planned_time)
                .flatten(),
            slot: input.slot,
            status: MealPlanStatus::Planned,
            components,
            participants,
            created_by: input.actor_id,
            updated_by: input.actor_id,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        recompute_statuses(&mut entry);
        self.plans.insert(&entry).await?;
        self.present(entry, &[], input.member_id).await
    }

    pub async fn set_participants(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: SetMealParticipants,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;
        validate_participants(&input.participants, &entry.components)?;

        for participant in &input.participants {
            let member = self
                .members
                .get(participant.member_id)
                .await?
                .ok_or_else(|| CoreError::not_found("household member", participant.member_id))?;
            if member.is_archived() {
                let mut errors = ValidationErrors::new();
                errors.push("participants", "Archived members cannot join a meal");
                return Err(errors.into());
            }
        }

        let now = self.clock.now();
        let mut participants = Vec::with_capacity(input.participants.len());
        for new_participant in &input.participants {
            let previous = entry
                .participants
                .iter()
                .find(|existing| existing.member_id == new_participant.member_id);
            if previous.is_none() {
                self.ensure_slot_free(
                    new_participant.member_id,
                    entry.planned_on,
                    entry.slot,
                    Some(entry.id),
                )
                .await?;
            }
            participants.push(merge_participant(
                previous,
                new_participant,
                &entry.components,
                now,
            ));
        }
        if let Some(removed) = entry.participants.iter().find(|existing| {
            !input
                .participants
                .iter()
                .any(|kept| kept.member_id == existing.member_id)
                && existing.allocations.iter().any(|a| a.status.is_resolved())
        }) {
            let _ = removed;
            return Err(CoreError::conflict(
                "A participant who has already eaten cannot be removed. Reopen their portion first.",
            ));
        }

        entry.participants = participants;
        entry.updated_by = input.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        recompute_statuses(&mut entry);
        commit_outcome(
            self.plans.set_participants(&entry, expected).await?,
            id,
            expected,
        )?;
        self.get(id).await
    }

    async fn active_member_ids(&self) -> Result<Vec<HouseholdMemberId>> {
        let page = self
            .members
            .list(&MemberQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        Ok(page
            .items
            .into_iter()
            .filter(|member| !member.is_archived())
            .map(|member| member.id)
            .collect())
    }

    async fn ensure_slot_free(
        &self,
        member_id: HouseholdMemberId,
        planned_on: Date,
        slot: MealSlot,
        ignore_entry: Option<MealPlanEntryId>,
    ) -> Result<()> {
        let clash = self
            .plans
            .list(&MealPlanQuery {
                member_id,
                from: planned_on,
                to: planned_on,
                include_participating: true,
            })
            .await?
            .into_iter()
            .any(|entry| entry.slot == slot && Some(entry.id) != ignore_entry);
        if clash {
            return Err(CoreError::conflict(
                "That meal slot already exists. Add food to the existing meal instead.",
            ));
        }
        Ok(())
    }

    pub async fn get(&self, id: MealPlanEntryId) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        let records = self.records_for_entry(entry.id).await?;
        self.present(entry, &records, None).await
    }

    pub async fn update(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        patch: MealPlanEntryPatch,
        actor_id: crate::domain::UserId,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;

        let now = self.clock.now();
        if let Some(components) = patch.components {
            validate_components(&components)?;
            let existing_items = entry
                .components
                .iter()
                .map(|component| component.item)
                .collect();
            self.validate_component_items(&components, &existing_items, actor_id)
                .await?;
            let owner = entry.member_id;
            entry.components = merge_components(&entry.components, components)?;
            sync_allocations(&mut entry, owner, now);
        }
        if let Some(planned_on) = patch.planned_on {
            ensure_not_past(&*self.clock, planned_on)?;
            entry.planned_on = planned_on;
        }
        if let Some(planned_time) = patch.planned_time {
            entry.planned_time = planned_time;
        }
        if let Some(slot) = patch.slot {
            entry.slot = slot;
        }
        if !entry.slot.allows_planned_time() {
            entry.planned_time = None;
        }
        entry.updated_by = actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        recompute_statuses(&mut entry);
        commit_outcome(self.plans.update(&entry, expected).await?, id, expected)?;
        self.get(id).await
    }

    pub async fn delete(&self, id: MealPlanEntryId, expected: Revision) -> Result<()> {
        let entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;
        commit_outcome(self.plans.delete(id, expected).await?, id, expected)
    }

    pub async fn mark_component_eaten(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        input: ConfirmMealPlanComponent,
    ) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_component_eaten_unchecked(id, component_id, expected, input)
            .await
    }

    pub async fn mark_component_eaten_unchecked(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        input: ConfirmMealPlanComponent,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, input.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_component_revision(component_id, expected, component.revision)?;
        let component_item = component.item;
        let planned_amount = component.amount;
        let old_component_revision = component.revision;

        require_allocation_planned(&entry, subject, component_id)?;

        let catalogue = self.catalogue_for([component_item]).await?;
        let planned = catalogue.resolve(component_item, &planned_amount);
        let actual = catalogue.resolve(component_item, &input.amount);
        if !actual.resolvable {
            let mut errors = ValidationErrors::new();
            errors.push("amount", "We cannot work out this item's nutrition");
            return Err(errors.into());
        }
        let now = self.clock.now();
        let record = ConsumptionRecord {
            id: Default::default(),
            member_id: subject,
            item: component_item,
            recorded_by: Some(input.actor_id),
            meal_plan_entry_id: Some(entry.id),
            meal_plan_component_id: Some(component_id),
            slot: entry.slot,
            amount: input.amount,
            consumed_on: input.consumed_on,
            consumed_at: input.consumed_at,
            nutrition: actual.nutrition.facts,
            quality: actual.nutrition.quality,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };

        set_allocation(
            &mut entry,
            subject,
            component_id,
            ParticipantStatus::Eaten,
            Some(record.id),
            Some(input.actor_id),
            Some(now),
        );
        let snapshot = MealPlanComponentSnapshot {
            item_name: planned.name,
            nutrition: planned.nutrition.facts,
            quality: planned.nutrition.quality,
        };
        recompute_statuses(&mut entry);
        let update = component_update(
            &entry,
            component_id,
            SnapshotOp::Set(&snapshot),
            old_component_revision,
            input.actor_id,
            now,
        );
        commit_component_outcome(
            self.plans
                .resolve_component(id, &update, &entry.participants, expected, Some(&record))
                .await?,
            component_id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn mark_component_not_eaten(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_component_not_eaten_unchecked(id, component_id, expected, actor)
            .await
    }

    pub async fn mark_component_not_eaten_unchecked(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, actor.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_component_revision(component_id, expected, component.revision)?;
        let component_item = component.item;
        let planned_amount = component.amount;
        let old_component_revision = component.revision;
        require_allocation_planned(&entry, subject, component_id)?;

        let catalogue = self.catalogue_for([component_item]).await?;
        let planned = catalogue.resolve(component_item, &planned_amount);
        let now = self.clock.now();

        set_allocation(
            &mut entry,
            subject,
            component_id,
            ParticipantStatus::NotEaten,
            None,
            Some(actor.actor_id),
            Some(now),
        );
        let snapshot = MealPlanComponentSnapshot {
            item_name: planned.name,
            nutrition: planned.nutrition.facts,
            quality: planned.nutrition.quality,
        };
        recompute_statuses(&mut entry);
        let update = component_update(
            &entry,
            component_id,
            SnapshotOp::Set(&snapshot),
            old_component_revision,
            actor.actor_id,
            now,
        );
        commit_component_outcome(
            self.plans
                .resolve_component(id, &update, &entry.participants, expected, None)
                .await?,
            component_id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn reopen_component(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, actor.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_component_revision(component_id, expected, component.revision)?;
        let old_component_revision = component.revision;

        let existing_allocation = entry.participant_for(subject).and_then(|p| {
            p.allocations
                .iter()
                .find(|a| a.component_id == component_id)
        });
        let allocation_resolved = existing_allocation
            .map(|a| a.status.is_resolved())
            .unwrap_or(false);
        if !allocation_resolved {
            return Err(CoreError::conflict("This item has not been resolved yet."));
        }
        let record_to_remove = existing_allocation.and_then(|a| a.consumption_record_id);

        let now = self.clock.now();
        if let Some(record_id) = record_to_remove {
            self.consumption.delete(record_id).await?;
        }
        set_allocation(
            &mut entry,
            subject,
            component_id,
            ParticipantStatus::Planned,
            None,
            None,
            None,
        );
        recompute_statuses(&mut entry);
        let still_eaten = entry.participants.iter().any(|p| {
            p.allocations
                .iter()
                .any(|a| a.component_id == component_id && a.status == ParticipantStatus::Eaten)
        });
        let snapshot_op = if still_eaten {
            SnapshotOp::Keep
        } else {
            SnapshotOp::Clear
        };
        let update = component_update(
            &entry,
            component_id,
            snapshot_op,
            old_component_revision,
            actor.actor_id,
            now,
        );
        commit_component_outcome(
            self.plans
                .reopen_component(id, &update, &entry.participants, expected)
                .await?,
            component_id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn mark_not_eaten(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_not_eaten_unchecked(id, expected, actor).await
    }

    pub async fn mark_not_eaten_unchecked(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        let subject = self.resolve_subject(&entry, actor.subject_member_id)?;
        require_subject_pending(&entry, subject)?;
        self.freeze(&mut entry).await?;
        let now = self.clock.now();
        for component_id in pending_component_ids(&entry, subject) {
            set_allocation(
                &mut entry,
                subject,
                component_id,
                ParticipantStatus::NotEaten,
                None,
                Some(actor.actor_id),
                Some(now),
            );
        }
        entry.updated_by = actor.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        recompute_statuses(&mut entry);
        commit_outcome(
            self.plans.resolve(&entry, expected, &[]).await?,
            id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn mark_eaten(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ConfirmMealPlanEntry,
    ) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_eaten_unchecked(id, expected, input).await
    }

    pub async fn mark_eaten_unchecked(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ConfirmMealPlanEntry,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        let subject = self.resolve_subject(&entry, input.subject_member_id)?;
        require_subject_pending(&entry, subject)?;
        let pending = pending_component_ids(&entry, subject);
        validate_actual_components(&pending, &input)?;
        self.freeze(&mut entry).await?;

        let catalogue = self
            .catalogue_for(entry.components.iter().map(|component| component.item))
            .await?;
        let actual_by_component: HashMap<_, _> = input
            .components
            .iter()
            .map(|actual| (actual.component_id, actual.amount))
            .collect();
        let now = self.clock.now();
        let mut records = Vec::with_capacity(input.components.len());
        for component_id in &pending {
            let component = find_component(&entry, *component_id)?.clone();
            let amount = actual_by_component[component_id];
            let scaled = catalogue.resolve(component.item, &amount);
            if !scaled.resolvable {
                let mut errors = ValidationErrors::new();
                errors.push(
                    format!("components.{component_id}.amount"),
                    "We cannot work out this item's nutrition",
                );
                return Err(errors.into());
            }
            let record = ConsumptionRecord {
                id: Default::default(),
                member_id: subject,
                item: component.item,
                recorded_by: Some(input.actor_id),
                meal_plan_entry_id: Some(entry.id),
                meal_plan_component_id: Some(component.id),
                slot: entry.slot,
                amount,
                consumed_on: input.consumed_on,
                consumed_at: input.consumed_at,
                nutrition: scaled.nutrition.facts,
                quality: scaled.nutrition.quality,
                revision: Revision::INITIAL,
                created_at: now,
                updated_at: now,
            };
            set_allocation(
                &mut entry,
                subject,
                *component_id,
                ParticipantStatus::Eaten,
                Some(record.id),
                Some(input.actor_id),
                Some(now),
            );
            records.push(record);
        }

        entry.updated_by = input.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        recompute_statuses(&mut entry);
        commit_outcome(
            self.plans.resolve(&entry, expected, &records).await?,
            id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn reopen(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        let subject = self.resolve_subject(&entry, actor.subject_member_id)?;
        let (resolved, record_ids): (Vec<MealPlanComponentId>, Vec<ConsumptionRecordId>) = entry
            .participant_for(subject)
            .map(|p| {
                let resolved = p
                    .allocations
                    .iter()
                    .filter(|a| a.status.is_resolved())
                    .map(|a| a.component_id)
                    .collect();
                let records = p
                    .allocations
                    .iter()
                    .filter_map(|a| a.consumption_record_id)
                    .collect();
                (resolved, records)
            })
            .unwrap_or_default();
        if resolved.is_empty() {
            return Err(CoreError::conflict("This meal has not been resolved yet."));
        }
        let now = self.clock.now();
        for record_id in record_ids {
            self.consumption.delete(record_id).await?;
        }
        for component_id in resolved {
            set_allocation(
                &mut entry,
                subject,
                component_id,
                ParticipantStatus::Planned,
                None,
                None,
                None,
            );
        }
        for component in &mut entry.components {
            let still_eaten = entry.participants.iter().any(|p| {
                p.allocations
                    .iter()
                    .any(|a| a.component_id == component.id && a.status == ParticipantStatus::Eaten)
            });
            if !still_eaten
                && derive_component_status(component.id, &entry.participants)
                    == MealPlanStatus::Planned
            {
                component.snapshot = None;
            }
        }
        entry.updated_by = actor.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        recompute_statuses(&mut entry);
        commit_outcome(self.plans.reopen(&entry, expected).await?, id, expected)?;
        self.get(id).await
    }

    pub async fn week(
        &self,
        member_id: crate::domain::HouseholdMemberId,
        week_start: Date,
    ) -> Result<MealPlanWeek> {
        let week_end = week_start + Duration::days(6);
        let entries = self
            .plans
            .list(&MealPlanQuery {
                member_id,
                from: week_start,
                to: week_end,
                include_participating: true,
            })
            .await?;
        let records = self
            .consumption
            .list_period(member_id, week_start, week_end)
            .await?;
        let targets = self.targets.list_for_member(member_id).await?;
        let mut records_by_entry: HashMap<MealPlanEntryId, Vec<ConsumptionRecord>> = HashMap::new();
        for record in &records {
            if let Some(entry_id) = record.meal_plan_entry_id {
                records_by_entry
                    .entry(entry_id)
                    .or_default()
                    .push(record.clone());
            }
        }

        let mut presented_by_date: BTreeMap<Date, Vec<MealPlanEntryView>> = BTreeMap::new();
        let mut items_by_slot: HashMap<(Date, MealSlot), Vec<(MealItemOrder, MealItem)>> =
            HashMap::new();
        for entry in entries {
            let linked = records_by_entry
                .get(&entry.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let date = entry.planned_on;
            let view = self.present(entry, linked, Some(member_id)).await?;
            for (item_date, order, item) in items_for_entry(&view) {
                let bucket = (item_date, view.entry.slot);
                items_by_slot.entry(bucket).or_default().push((order, item));
            }
            presented_by_date.entry(date).or_default().push(view);
        }
        let logged_catalogue = self
            .catalogue_for(
                records
                    .iter()
                    .filter(|record| record.meal_plan_component_id.is_none())
                    .map(|record| record.item),
            )
            .await?;
        for record in &records {
            if record.meal_plan_component_id.is_some() {
                continue;
            }
            let item_name = logged_catalogue.name_of(record.item);
            let bucket = (record.consumed_on, record.slot);
            items_by_slot
                .entry(bucket)
                .or_default()
                .push((record.id.as_uuid(), logged_item(record, item_name)));
        }

        let mut days = Vec::with_capacity(7);
        for offset in 0..7 {
            let date = week_start + Duration::days(offset);
            let mut day_entries = presented_by_date.remove(&date).unwrap_or_default();
            day_entries.sort_by_key(|view| {
                (
                    view.entry.slot.order(),
                    view.entry.planned_time,
                    view.entry.created_at,
                    view.entry.id,
                )
            });
            let actual = summary(records.iter().filter(|record| record.consumed_on == date));
            let remaining_planned = summary_from_views(day_entries.iter());
            let projected = combine_summaries(&actual, &remaining_planned);
            let target = resolve_on(&targets, date).map(|target| target.goals.clone());
            let slots = MealSlot::ALL
                .into_iter()
                .map(|slot| {
                    let mut items = items_by_slot.remove(&(date, slot)).unwrap_or_default();
                    items.sort_by_key(|(order, _)| *order);
                    let items: Vec<_> = items.into_iter().map(|(_, item)| item).collect();
                    let nutrition = item_summary(&items);
                    MealSlotView {
                        slot,
                        items,
                        nutrition,
                    }
                })
                .collect();
            days.push(MealPlanDay {
                date,
                entries: day_entries,
                slots,
                actual,
                remaining_planned,
                projected,
                target,
            });
        }

        let actual = combine_many(days.iter().map(|day| &day.actual));
        let remaining_planned = combine_many(days.iter().map(|day| &day.remaining_planned));
        let projected = combine_summaries(&actual, &remaining_planned);
        let (target, insufficient_target_coverage) =
            weekly_goals(days.iter().map(|day| day.target.as_ref()));
        Ok(MealPlanWeek {
            member_id,
            week_start,
            week_end,
            days,
            actual,
            remaining_planned,
            projected,
            target,
            insufficient_target_coverage,
        })
    }

    async fn get_entry(&self, id: MealPlanEntryId) -> Result<MealPlanEntry> {
        self.plans
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))
    }

    fn resolve_subject(
        &self,
        entry: &MealPlanEntry,
        requested: Option<HouseholdMemberId>,
    ) -> Result<HouseholdMemberId> {
        entry.subject_or_owner(requested).ok_or_else(|| {
            CoreError::conflict("This meal has no participant to record an outcome against.")
        })
    }

    async fn validate_component_items(
        &self,
        components: &[NewMealPlanComponent],
        archived_allowed: &HashSet<MealItemRef>,
        actor_id: UserId,
    ) -> Result<()> {
        for (index, component) in components.iter().enumerate() {
            let mut errors = ValidationErrors::new();
            match component.item {
                MealItemRef::Product { product_id } => {
                    let product = self
                        .products
                        .get(product_id)
                        .await?
                        .ok_or_else(|| CoreError::not_found(PRODUCT, product_id))?;
                    if product.is_archived() && !archived_allowed.contains(&component.item) {
                        errors.push(
                            format!("components.{index}.item"),
                            "That product is archived",
                        );
                    }
                    if let Err(error) = component.amount.resolve(&product) {
                        errors.push(format!("components.{index}.amount"), error.to_string());
                    }
                }
                MealItemRef::Recipe { recipe_id } => {
                    let recipe = self
                        .recipes
                        .get(recipe_id)
                        .await?
                        .filter(|recipe| recipe.owner_id == actor_id)
                        .ok_or_else(|| CoreError::not_found(RECIPE, recipe_id))?;
                    if recipe.is_archived() && !archived_allowed.contains(&component.item) {
                        errors.push(
                            format!("components.{index}.item"),
                            "That recipe is archived",
                        );
                    }
                }
            }
            errors.into_result()?;
        }
        Ok(())
    }

    async fn freeze(&self, entry: &mut MealPlanEntry) -> Result<()> {
        let catalogue = self
            .catalogue_for(entry.components.iter().map(|component| component.item))
            .await?;
        for component in &mut entry.components {
            if component.snapshot.is_some() {
                continue;
            }
            let resolved = catalogue.resolve(component.item, &component.amount);
            if !resolved.resolvable {
                let mut errors = ValidationErrors::new();
                errors.push(
                    "components.amount",
                    "We cannot work out this item's nutrition",
                );
                return errors.into_result();
            }
            component.snapshot = Some(MealPlanComponentSnapshot {
                item_name: resolved.name,
                nutrition: resolved.nutrition.facts,
                quality: resolved.nutrition.quality,
            });
        }
        Ok(())
    }

    async fn catalogue_for(
        &self,
        items: impl IntoIterator<Item = MealItemRef>,
    ) -> Result<ItemCatalogue> {
        let mut product_ids: Vec<ProductId> = Vec::new();
        let mut recipe_ids: Vec<RecipeId> = Vec::new();
        for item in items {
            match item {
                MealItemRef::Product { product_id } => product_ids.push(product_id),
                MealItemRef::Recipe { recipe_id } => recipe_ids.push(recipe_id),
            }
        }

        let recipes = self.recipes.get_many(&recipe_ids).await?;
        let requirements: Vec<&RecipeRequirement> = recipes
            .iter()
            .flat_map(|recipe| {
                recipe
                    .components
                    .iter()
                    .map(|component| &component.requirement)
            })
            .collect();
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;

        let products: HashMap<ProductId, Product> = self
            .products
            .get_many(&product_ids)
            .await?
            .into_iter()
            .map(|product| (product.id, product))
            .collect();
        let recipes: HashMap<RecipeId, RecipeCard> = recipes
            .into_iter()
            .map(|recipe| (recipe.id, recipe_card(&recipe, &fulfilments)))
            .collect();

        Ok(ItemCatalogue { products, recipes })
    }

    async fn present(
        &self,
        entry: MealPlanEntry,
        records: &[ConsumptionRecord],
        requested_subject: Option<HouseholdMemberId>,
    ) -> Result<MealPlanEntryView> {
        let subject = entry.subject_or_owner(requested_subject);
        let catalogue = self
            .catalogue_for(entry.components.iter().map(|component| component.item))
            .await?;
        let records_by_key: HashMap<(MealPlanComponentId, HouseholdMemberId), ConsumptionRecord> =
            records
                .iter()
                .filter_map(|record| {
                    record
                        .meal_plan_component_id
                        .map(|component_id| ((component_id, record.member_id), record.clone()))
                })
                .collect();

        let component_ids: HashSet<MealPlanComponentId> = entry
            .components
            .iter()
            .map(|component| component.id)
            .collect();

        let mut needs_attention = false;
        let mut components = Vec::with_capacity(entry.components.len());
        for component in &entry.components {
            let subject_alloc = subject.and_then(|member| {
                entry.participant_for(member).and_then(|participant| {
                    participant
                        .allocations
                        .iter()
                        .find(|a| a.component_id == component.id)
                        .map(|a| a.allocated)
                })
            });
            let display_amount = subject_alloc.unwrap_or(component.amount);
            let subject_record = subject
                .and_then(|member| records_by_key.get(&(component.id, member)))
                .cloned();

            let (item_name, nutrition, quality) = if let Some(record) = &subject_record {
                let name = component
                    .snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.item_name.clone())
                    .unwrap_or_else(|| catalogue.name_of(component.item));
                (name, record.nutrition.clone(), record.quality)
            } else if let Some(snapshot) = &component.snapshot {
                (
                    snapshot.item_name.clone(),
                    snapshot.scaled_to(&component.amount, &display_amount),
                    snapshot.quality,
                )
            } else {
                let resolved = catalogue.resolve(component.item, &display_amount);
                if !resolved.resolvable {
                    needs_attention = true;
                }
                (
                    resolved.name,
                    resolved.nutrition.facts,
                    resolved.nutrition.quality,
                )
            };

            let outcomes = outcomes_for_component(&entry, &records_by_key, component.id);
            let preparation = preparation_for(&component.amount, &outcomes);
            let subject_status = subject
                .and_then(|member| entry.participant_for(member))
                .and_then(|participant| {
                    participant
                        .allocations
                        .iter()
                        .find(|a| a.component_id == component.id)
                        .map(|a| participant_status_to_meal(a.status))
                })
                .unwrap_or(component.status);

            components.push(MealPlanComponentView {
                component: component.clone(),
                item_name,
                nutrition,
                quality,
                consumption_record: subject_record,
                preparation,
                subject_status,
            });
        }

        let mut participant_views = Vec::with_capacity(entry.participants.len());
        for participant in &entry.participants {
            let display_name = self
                .members
                .get(participant.member_id)
                .await?
                .map(|member| member.display_name)
                .unwrap_or_default();
            let participant_records: Vec<&ConsumptionRecord> = records
                .iter()
                .filter(|record| {
                    record.member_id == participant.member_id
                        && record
                            .meal_plan_component_id
                            .is_some_and(|id| component_ids.contains(&id))
                })
                .collect();
            participant_views.push(MealParticipantView {
                member_id: participant.member_id,
                display_name,
                status: derive_participant_status(participant),
                allocations: participant.allocations.clone(),
                nutrition: summary(participant_records.into_iter()),
            });
        }

        let planned = summary_components(
            components
                .iter()
                .filter(|component| component.subject_status == MealPlanStatus::Planned),
        );
        let subject_records: Vec<&ConsumptionRecord> = subject
            .map(|member| {
                records
                    .iter()
                    .filter(|record| record.member_id == member)
                    .collect()
            })
            .unwrap_or_default();
        let actual = if subject_records.is_empty() {
            None
        } else {
            Some(summary(subject_records.into_iter()))
        };

        Ok(MealPlanEntryView {
            entry,
            subject_member_id: subject,
            components,
            participants: participant_views,
            planned,
            actual,
            needs_attention,
        })
    }

    async fn records_for_entry(&self, entry_id: MealPlanEntryId) -> Result<Vec<ConsumptionRecord>> {
        self.consumption.list_for_meal_plan_entry(entry_id).await
    }
}

fn allocation_for_kind(component: &MealPlanComponent, full: bool) -> ConsumedAmount {
    if full {
        return component.amount;
    }
    match component.amount {
        ConsumedAmount::Servings(_) => ConsumedAmount::Servings(Decimal::ONE),
        other => other,
    }
}

fn build_participant(
    member_id: HouseholdMemberId,
    components: &[MealPlanComponent],
    now: OffsetDateTime,
    full: bool,
) -> MealParticipant {
    MealParticipant {
        id: MealParticipantId::new(),
        member_id,
        allocations: components
            .iter()
            .map(|component| MealParticipantAllocation {
                id: MealParticipantAllocationId::new(),
                component_id: component.id,
                allocated: allocation_for_kind(component, full),
                status: ParticipantStatus::Planned,
                consumption_record_id: None,
                resolved_by: None,
                resolved_at: None,
            })
            .collect(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

fn merge_participant(
    previous: Option<&MealParticipant>,
    new_participant: &NewMealParticipant,
    components: &[MealPlanComponent],
    now: OffsetDateTime,
) -> MealParticipant {
    let requested: HashMap<MealPlanComponentId, ConsumedAmount> = new_participant
        .allocations
        .iter()
        .map(|allocation| (allocation.component_id, allocation.allocated))
        .collect();
    let allocations = components
        .iter()
        .map(|component| {
            let existing = previous.and_then(|participant| {
                participant
                    .allocations
                    .iter()
                    .find(|a| a.component_id == component.id)
            });
            let allocated = requested
                .get(&component.id)
                .copied()
                .or_else(|| existing.map(|a| a.allocated))
                .unwrap_or_else(|| allocation_for_kind(component, false));
            MealParticipantAllocation {
                id: existing.map(|a| a.id).unwrap_or_default(),
                component_id: component.id,
                allocated,
                status: existing
                    .map(|a| a.status)
                    .unwrap_or(ParticipantStatus::Planned),
                consumption_record_id: existing.and_then(|a| a.consumption_record_id),
                resolved_by: existing.and_then(|a| a.resolved_by),
                resolved_at: existing.and_then(|a| a.resolved_at),
            }
        })
        .collect();
    MealParticipant {
        id: previous
            .map(|p| p.id)
            .or(new_participant.id)
            .unwrap_or_default(),
        member_id: new_participant.member_id,
        allocations,
        revision: previous.map(|p| p.revision).unwrap_or(Revision::INITIAL),
        created_at: previous.map(|p| p.created_at).unwrap_or(now),
        updated_at: now,
    }
}

fn sync_allocations(
    entry: &mut MealPlanEntry,
    owner: Option<HouseholdMemberId>,
    now: OffsetDateTime,
) {
    let components = entry.components.clone();
    for participant in &mut entry.participants {
        let full = Some(participant.member_id) == owner;
        participant.allocations.retain(|allocation| {
            components
                .iter()
                .any(|component| component.id == allocation.component_id)
        });
        for component in &components {
            if !participant
                .allocations
                .iter()
                .any(|allocation| allocation.component_id == component.id)
            {
                participant.allocations.push(MealParticipantAllocation {
                    id: MealParticipantAllocationId::new(),
                    component_id: component.id,
                    allocated: allocation_for_kind(component, full),
                    status: ParticipantStatus::Planned,
                    consumption_record_id: None,
                    resolved_by: None,
                    resolved_at: None,
                });
            }
        }
        participant.updated_at = now;
    }
}

fn participant_status_to_meal(status: ParticipantStatus) -> MealPlanStatus {
    match status {
        ParticipantStatus::Planned => MealPlanStatus::Planned,
        ParticipantStatus::Eaten => MealPlanStatus::Eaten,
        ParticipantStatus::NotEaten => MealPlanStatus::NotEaten,
    }
}

fn recompute_statuses(entry: &mut MealPlanEntry) {
    let participants = entry.participants.clone();
    for component in &mut entry.components {
        let status = derive_component_status(component.id, &participants);
        component.status = status;
        if status == MealPlanStatus::Planned {
            component.resolved_by = None;
            component.resolved_at = None;
        } else {
            let resolved: Vec<&MealParticipantAllocation> = participants
                .iter()
                .flat_map(|participant| participant.allocations.iter())
                .filter(|allocation| {
                    allocation.component_id == component.id && allocation.status.is_resolved()
                })
                .collect();
            component.resolved_by = resolved
                .iter()
                .find_map(|allocation| allocation.resolved_by);
            component.resolved_at = resolved.iter().filter_map(|a| a.resolved_at).max();
        }
    }
    let entry_status = derive_entry_status(&participants);
    entry.status = entry_status;
    if entry_status == MealPlanStatus::Planned {
        entry.resolved_by = None;
        entry.resolved_at = None;
    } else {
        let resolved: Vec<&MealParticipantAllocation> = participants
            .iter()
            .flat_map(|participant| participant.allocations.iter())
            .filter(|allocation| allocation.status.is_resolved())
            .collect();
        entry.resolved_by = resolved
            .iter()
            .find_map(|allocation| allocation.resolved_by);
        entry.resolved_at = resolved.iter().filter_map(|a| a.resolved_at).max();
    }
}

fn set_allocation(
    entry: &mut MealPlanEntry,
    member_id: HouseholdMemberId,
    component_id: MealPlanComponentId,
    status: ParticipantStatus,
    record_id: Option<ConsumptionRecordId>,
    resolved_by: Option<UserId>,
    resolved_at: Option<OffsetDateTime>,
) {
    if let Some(participant) = entry
        .participants
        .iter_mut()
        .find(|participant| participant.member_id == member_id)
        && let Some(allocation) = participant
            .allocations
            .iter_mut()
            .find(|allocation| allocation.component_id == component_id)
    {
        allocation.status = status;
        allocation.consumption_record_id = record_id;
        allocation.resolved_by = resolved_by;
        allocation.resolved_at = resolved_at;
    }
}

fn outcomes_for_component(
    entry: &MealPlanEntry,
    records_by_key: &HashMap<(MealPlanComponentId, HouseholdMemberId), ConsumptionRecord>,
    component_id: MealPlanComponentId,
) -> Vec<AllocationOutcome> {
    entry
        .participants
        .iter()
        .filter_map(|participant| {
            participant
                .allocations
                .iter()
                .find(|allocation| allocation.component_id == component_id)
                .map(|allocation| AllocationOutcome {
                    allocated: allocation.allocated,
                    status: allocation.status,
                    confirmed: records_by_key
                        .get(&(component_id, participant.member_id))
                        .map(|record| record.amount),
                })
        })
        .collect()
}

fn pending_component_ids(
    entry: &MealPlanEntry,
    member_id: HouseholdMemberId,
) -> Vec<MealPlanComponentId> {
    entry
        .participant_for(member_id)
        .map(|participant| {
            participant
                .allocations
                .iter()
                .filter(|allocation| allocation.status == ParticipantStatus::Planned)
                .map(|allocation| allocation.component_id)
                .collect()
        })
        .unwrap_or_default()
}

fn component_update<'a>(
    entry: &MealPlanEntry,
    component_id: MealPlanComponentId,
    snapshot: SnapshotOp<'a>,
    old_revision: Revision,
    actor_id: UserId,
    now: OffsetDateTime,
) -> MealPlanComponentUpdate<'a> {
    let component = entry
        .components
        .iter()
        .find(|component| component.id == component_id)
        .expect("component present");
    MealPlanComponentUpdate {
        id: component_id,
        status: component.status,
        snapshot,
        resolved_by: component.resolved_by,
        resolved_at: component.resolved_at,
        revision: old_revision.next(),
        entry_status: entry.status,
        entry_resolved_by: entry.resolved_by,
        entry_resolved_at: entry.resolved_at,
        actor_id,
        now,
    }
}

fn find_component(
    entry: &MealPlanEntry,
    component_id: MealPlanComponentId,
) -> Result<&MealPlanComponent> {
    entry
        .components
        .iter()
        .find(|component| component.id == component_id)
        .ok_or_else(|| CoreError::not_found(MEAL_PLAN_COMPONENT, component_id))
}

fn require_allocation_planned(
    entry: &MealPlanEntry,
    member_id: HouseholdMemberId,
    component_id: MealPlanComponentId,
) -> Result<()> {
    let participant = entry.participant_for(member_id).ok_or_else(|| {
        CoreError::conflict("That household member is not a participant in this meal.")
    })?;
    let allocation = participant
        .allocations
        .iter()
        .find(|allocation| allocation.component_id == component_id)
        .ok_or_else(|| {
            CoreError::conflict("That household member has no portion of this item to resolve.")
        })?;
    if allocation.status == ParticipantStatus::Planned {
        Ok(())
    } else {
        Err(CoreError::conflict("This item has already been resolved."))
    }
}

fn require_subject_pending(entry: &MealPlanEntry, member_id: HouseholdMemberId) -> Result<()> {
    let participant = entry.participant_for(member_id).ok_or_else(|| {
        CoreError::conflict("That household member is not a participant in this meal.")
    })?;
    if participant
        .allocations
        .iter()
        .any(|allocation| allocation.status == ParticipantStatus::Planned)
    {
        Ok(())
    } else {
        Err(CoreError::conflict(
            "This meal has no remaining planned items.",
        ))
    }
}

fn make_components(input: Vec<NewMealPlanComponent>) -> Vec<MealPlanComponent> {
    input
        .into_iter()
        .enumerate()
        .map(|(position, component)| MealPlanComponent {
            id: component.id.unwrap_or_default(),
            item: component.item,
            amount: component.amount,
            position: i32::try_from(position).unwrap_or(i32::MAX),
            snapshot: None,
            status: MealPlanStatus::Planned,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            display_order: uuid::Uuid::now_v7(),
        })
        .collect()
}

fn merge_components(
    existing: &[MealPlanComponent],
    input: Vec<NewMealPlanComponent>,
) -> Result<Vec<MealPlanComponent>> {
    let mut used = HashSet::new();
    let mut errors = ValidationErrors::new();
    let components = input
        .into_iter()
        .enumerate()
        .map(|(position, component)| {
            let Some(id) = component.id else {
                return MealPlanComponent {
                    id: Default::default(),
                    item: component.item,
                    amount: component.amount,
                    position: i32::try_from(position).unwrap_or(i32::MAX),
                    snapshot: None,
                    status: MealPlanStatus::Planned,
                    resolved_by: None,
                    resolved_at: None,
                    revision: Revision::INITIAL,
                    display_order: uuid::Uuid::now_v7(),
                };
            };
            if !used.insert(id) {
                errors.push(
                    format!("components.{position}.id"),
                    "Use each component once",
                );
            }
            let Some(previous) = existing.iter().find(|candidate| candidate.id == id) else {
                errors.push(
                    format!("components.{position}.id"),
                    "That component does not belong to this meal",
                );
                return MealPlanComponent {
                    id,
                    item: component.item,
                    amount: component.amount,
                    position: i32::try_from(position).unwrap_or(i32::MAX),
                    snapshot: None,
                    status: MealPlanStatus::Planned,
                    resolved_by: None,
                    resolved_at: None,
                    revision: Revision::INITIAL,
                    display_order: uuid::Uuid::now_v7(),
                };
            };
            let position = i32::try_from(position).unwrap_or(i32::MAX);
            let changed = previous.item != component.item
                || previous.amount != component.amount
                || previous.position != position;
            MealPlanComponent {
                id,
                item: component.item,
                amount: component.amount,
                position,
                snapshot: previous.snapshot.clone(),
                status: previous.status,
                resolved_by: previous.resolved_by,
                resolved_at: previous.resolved_at,
                revision: if changed {
                    previous.revision.next()
                } else {
                    previous.revision
                },
                display_order: previous.display_order,
            }
        })
        .collect();
    errors.into_result()?;
    Ok(components)
}

fn validate_actual_components(
    pending: &[MealPlanComponentId],
    input: &ConfirmMealPlanEntry,
) -> Result<()> {
    let expected: HashSet<_> = pending.iter().copied().collect();
    let actual: HashSet<_> = input
        .components
        .iter()
        .map(|component| component.component_id)
        .collect();
    let mut errors = ValidationErrors::new();
    if input.components.len() != expected.len() || actual != expected {
        errors.push("components", "Confirm every planned product exactly once");
    }
    for (index, component) in input.components.iter().enumerate() {
        if component.amount.value() <= rust_decimal::Decimal::ZERO {
            errors.push(
                format!("components.{index}.amount"),
                "Must be more than zero",
            );
        }
    }
    errors.into_result()
}

fn ensure_not_past(clock: &dyn Clock, planned_on: Date) -> Result<()> {
    let earliest = clock.now().date() - Duration::days(1);
    if planned_on < earliest {
        let mut errors = ValidationErrors::new();
        errors.push("planned_on", "Plans cannot be dated in the past");
        return errors.into_result();
    }
    Ok(())
}

fn ensure_due(clock: &dyn Clock, planned_on: Date) -> Result<()> {
    let latest = clock.now().date() + Duration::days(1);
    if planned_on > latest {
        return Err(CoreError::conflict("This meal is not due yet."));
    }
    Ok(())
}

fn require_planned(entry: &MealPlanEntry) -> Result<()> {
    if entry.status == MealPlanStatus::Planned {
        Ok(())
    } else {
        Err(CoreError::conflict(
            "Resolved meal plans cannot be changed.",
        ))
    }
}

fn require_revision(id: MealPlanEntryId, expected: Revision, actual: Revision) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_ENTRY,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn require_component_revision(
    id: MealPlanComponentId,
    expected: Revision,
    actual: Revision,
) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_COMPONENT,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn commit_component_outcome(
    outcome: UpdateOutcome,
    id: MealPlanComponentId,
    expected: Revision,
) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_COMPONENT,
            id: id.to_string(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(MEAL_PLAN_COMPONENT, id)),
    }
}

fn commit_outcome(outcome: UpdateOutcome, id: MealPlanEntryId, expected: Revision) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_ENTRY,
            id: id.to_string(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(MEAL_PLAN_ENTRY, id)),
    }
}

fn items_for_entry(view: &MealPlanEntryView) -> Vec<(Date, MealItemOrder, MealItem)> {
    view.components
        .iter()
        .map(|component| {
            let order = component.component.display_order;
            let source = MealItemSource::Planned {
                entry_id: view.entry.id,
                component_id: component.component.id,
            };
            match &component.consumption_record {
                Some(record) => (
                    record.consumed_on,
                    order,
                    MealItem {
                        source,
                        record_id: Some(record.id),
                        status: MealPlanStatus::Eaten,
                        item: component.component.item,
                        item_name: component.item_name.clone(),
                        amount: record.amount,
                        planned_amount: (record.amount != component.component.amount)
                            .then_some(component.component.amount),
                        planned_on: (record.consumed_on != view.entry.planned_on)
                            .then_some(view.entry.planned_on),
                        at: None,
                        consumed_at: record.consumed_at,
                        nutrition: record.nutrition.clone(),
                        quality: record.quality,
                        needs_attention: view.needs_attention,
                        revision: component.component.revision,
                        record_revision: Some(record.revision),
                    },
                ),
                None => (
                    view.entry.planned_on,
                    order,
                    MealItem {
                        source,
                        record_id: None,
                        status: component.subject_status,
                        item: component.component.item,
                        item_name: component.item_name.clone(),
                        amount: component
                            .preparation
                            .allocated
                            .filter(|_| component.subject_status != MealPlanStatus::NotEaten)
                            .map(|_| component.component.amount)
                            .unwrap_or(component.component.amount),
                        planned_amount: None,
                        planned_on: None,
                        at: view.entry.planned_time,
                        consumed_at: None,
                        nutrition: component.nutrition.clone(),
                        quality: component.quality,
                        needs_attention: view.needs_attention,
                        revision: component.component.revision,
                        record_revision: None,
                    },
                ),
            }
        })
        .collect()
}

fn logged_item(record: &ConsumptionRecord, item_name: String) -> MealItem {
    MealItem {
        source: MealItemSource::Logged {
            record_id: record.id,
        },
        record_id: Some(record.id),
        status: MealPlanStatus::Eaten,
        item: record.item,
        item_name,
        amount: record.amount,
        planned_amount: None,
        planned_on: None,
        at: None,
        consumed_at: record.consumed_at,
        nutrition: record.nutrition.clone(),
        quality: record.quality,
        needs_attention: false,
        revision: record.revision,
        record_revision: Some(record.revision),
    }
}

fn item_summary(items: &[MealItem]) -> NutritionSummary {
    let included: Vec<_> = items
        .iter()
        .filter(|item| item.status != MealPlanStatus::NotEaten)
        .collect();
    NutritionSummary {
        nutrition: sum_nutrition(included.iter().map(|item| &item.nutrition)),
        unknown_count: included
            .iter()
            .filter(|item| item.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: included
            .iter()
            .filter(|item| item.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary_components<'a>(
    components: impl Iterator<Item = &'a MealPlanComponentView>,
) -> NutritionSummary {
    let components: Vec<_> = components.collect();
    NutritionSummary {
        nutrition: sum_nutrition(components.iter().map(|component| &component.nutrition)),
        unknown_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary<'a>(records: impl Iterator<Item = &'a ConsumptionRecord>) -> NutritionSummary {
    let records: Vec<_> = records.collect();
    NutritionSummary {
        nutrition: sum_nutrition(records.iter().map(|record| &record.nutrition)),
        unknown_count: records
            .iter()
            .filter(|record| record.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: records
            .iter()
            .filter(|record| record.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary_from_views<'a>(views: impl Iterator<Item = &'a MealPlanEntryView>) -> NutritionSummary {
    let components: Vec<_> = views
        .flat_map(|view| view.components.iter())
        .filter(|component| component.subject_status == MealPlanStatus::Planned)
        .collect();
    NutritionSummary {
        nutrition: sum_nutrition(components.iter().map(|component| &component.nutrition)),
        unknown_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn combine_many<'a>(summaries: impl Iterator<Item = &'a NutritionSummary>) -> NutritionSummary {
    let summaries: Vec<_> = summaries.collect();
    NutritionSummary {
        nutrition: sum_nutrition(summaries.iter().map(|summary| &summary.nutrition)),
        unknown_count: summaries.iter().map(|summary| summary.unknown_count).sum(),
        partial_count: summaries.iter().map(|summary| summary.partial_count).sum(),
    }
}

fn combine_summaries(left: &NutritionSummary, right: &NutritionSummary) -> NutritionSummary {
    combine_many([left, right].into_iter())
}

fn weekly_goals<'a>(
    daily: impl Iterator<Item = Option<&'a NutritionGoals>>,
) -> (Option<NutritionGoals>, Vec<String>) {
    let daily: Vec<Option<&NutritionGoals>> = daily.collect();
    let day_count = daily.len();
    let mut goals = NutritionGoals::default();
    let mut insufficient = Vec::new();
    for key in NUTRIENT_KEYS {
        let values: Vec<Decimal> = daily
            .iter()
            .filter_map(|day| day.and_then(|day| day.get(key)))
            .collect();
        let covered = values.len();
        if day_count > 0 && covered == day_count {
            goals.set(key, Some(values.iter().copied().sum()));
        } else if covered > 0 {
            insufficient.push(key.to_string());
        }
    }
    let target = if goals.is_empty() { None } else { Some(goals) };
    (target, insufficient)
}

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
