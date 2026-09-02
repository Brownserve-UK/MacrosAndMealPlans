use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use time::{Date, Duration, OffsetDateTime, Time};

use rust_decimal::Decimal;

use crate::domain::{
    AllocationOutcome, Assumption, AssumptionRules, ComponentPreparation, ConfirmMealPlanComponent,
    ConfirmMealPlanEntry, ConsumedAmount, ConsumedNutrition, ConsumptionRecord,
    ConsumptionRecordId, HouseholdMemberId, MealGuestAllocation, MealGuestGroup, MealItemRef,
    MealOptOut, MealParticipant, MealParticipantAllocation, MealParticipantAllocationId,
    MealParticipantId, MealPlanComponent, MealPlanComponentId, MealPlanComponentSnapshot,
    MealPlanEntry, MealPlanEntryId, MealPlanEntryPatch, MealPlanScope, MealPlanStatus, MealSlot,
    NUTRIENT_KEYS, NewMealGuestGroup, NewMealParticipant, NewMealPlanComponent, NewMealPlanEntry,
    NutritionFacts, NutritionGoals, NutritionQuality, OutcomeActor, ParticipantStatus, Portioning,
    Product, ProductId, Recipe, RecipeId, RecipeRequirement, RecipeVisibility, ReplacementItem,
    ReviewMealOutcomes, ReviewedMealOutcome, Revision, SetMealParticipants, SlotAttendance, UserId,
    derive_component_status, derive_participant_status, equal_split, nutrition_for,
    preparation_for, recipe_nutrition, recipe_nutrition_for, resolve_on, sum_nutrition,
    validate_components, validate_participants,
};
use crate::domain::{StockEffectSource, StockOutcome};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, ConsumptionRecordRepository, HouseholdMemberRepository, HouseholdSettingsRepository,
    IngredientRepository, MealPlanComponentUpdate, MealPlanQuery, MealPlanRepository, MemberQuery,
    NutritionTargetRepository, PageRequest, ProductRepository, RecipeRepository, SnapshotOp,
    StockDeduction, StockRelease, StockWrite, UpdateOutcome,
};

use super::fulfilment::{RecipeFulfilments, RecipeWant, expand_recipe};
use super::stock_effects::{
    StockAffected, component_release, name_outcomes, product_deduction, record_deduction,
    requirement_deduction,
};

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
    pub status: MealPlanStatus,
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
pub struct NeedsReview {
    pub personal: Vec<MealPlanEntryView>,
    pub household: Vec<MealPlanEntryView>,
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
    pub assumption: Assumption,
    pub status: MealPlanStatus,
    pub subject_status: MealPlanStatus,
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
    definitions: HashMap<RecipeId, Recipe>,
    fulfilments: RecipeFulfilments,
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

    fn recipe_wants(&self, recipe_id: RecipeId, amount: &ConsumedAmount) -> Vec<RecipeWant> {
        let Some(recipe) = self.definitions.get(&recipe_id) else {
            return Vec::new();
        };
        let ConsumedAmount::Servings(servings) = *amount else {
            return Vec::new();
        };
        expand_recipe(recipe, servings, &self.fulfilments).wants
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
    ingredients: Arc<dyn IngredientRepository>,
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
        ingredients: Arc<dyn IngredientRepository>,
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
            ingredients,
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
        if input.scope == MealPlanScope::Household && input.slot == MealSlot::Snacks {
            return Err(CoreError::conflict(
                "Snacks stay on your own planner. Household planning covers breakfast, lunch and dinner.",
            ));
        }
        if input.scope == MealPlanScope::Member {
            let owner = input.member_id.expect("personal meal owner checked");
            if !input.guest_groups.is_empty()
                || input.participants.as_ref().is_some_and(|participants| {
                    participants.len() != 1 || participants[0].member_id != owner
                })
            {
                return Err(CoreError::conflict(
                    "A personal meal can only contain its owner.",
                ));
            }
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

        let mut portioning = input.portioning;
        if input
            .participants
            .as_deref()
            .is_some_and(has_explicit_allocations)
        {
            portioning = Portioning::Custom;
        }

        let participants = if let Some(requested) = &input.participants {
            validate_participants(requested, &components)?;
            let mut result = Vec::with_capacity(requested.len());
            for participant in requested {
                let member = self
                    .members
                    .get(participant.member_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::not_found("household member", participant.member_id)
                    })?;
                if member.is_archived() {
                    let mut errors = ValidationErrors::new();
                    errors.push("participants", "Archived members cannot join a meal");
                    return Err(errors.into());
                }
                result.push(merge_participant(None, participant, &components, now));
            }
            result
        } else {
            let mut result = Vec::new();
            if let Some(member_id) = owner_member_id {
                result.push(build_participant(member_id, &components, now, true));
            }
            for member_id in extra_member_ids {
                result.push(build_participant(member_id, &components, now, false));
            }
            result
        };
        validate_guest_groups(&input.guest_groups, &components)?;
        let guest_groups: Vec<_> = input
            .guest_groups
            .iter()
            .map(|group| merge_guest_group(&[], group, now))
            .collect();
        require_household_attendance(input.scope, &participants, &guest_groups)?;
        for member_id in participants.iter().map(|p| p.member_id).collect::<Vec<_>>() {
            self.ensure_slot_free(
                member_id,
                input.planned_on,
                input.slot,
                input.planned_time,
                None,
            )
            .await?;
        }

        let mut entry = MealPlanEntry {
            id: input.id.unwrap_or_default(),
            scope: input.scope,
            member_id: input.member_id,
            planned_on: input.planned_on,
            planned_time: input.planned_time,
            slot: input.slot,
            portioning,
            components,
            participants,
            guest_groups,
            opted_out: Vec::new(),
            created_by: input.actor_id,
            updated_by: input.actor_id,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        apply_equal_portioning(&mut entry);
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
        require_editable(&entry)?;
        validate_participants(&input.participants, &entry.components)?;
        validate_guest_groups(&input.guest_groups, &entry.components)?;
        require_household_attendance(entry.scope, &input.participants, &input.guest_groups)?;

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
                if entry.has_opted_out(new_participant.member_id) {
                    return Err(CoreError::conflict(
                        "This member has opted out of this meal. They can rejoin it from their own planner.",
                    ));
                }
                self.ensure_slot_free(
                    new_participant.member_id,
                    entry.planned_on,
                    entry.slot,
                    entry.planned_time,
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
        entry.guest_groups = input
            .guest_groups
            .iter()
            .map(|group| merge_guest_group(&entry.guest_groups, group, now))
            .collect();
        if has_explicit_allocations(&input.participants) {
            entry.portioning = Portioning::Custom;
        }
        entry.updated_by = input.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        apply_equal_portioning(&mut entry);
        commit_outcome(
            self.plans.set_participants(&entry, expected).await?,
            id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn opt_out(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor_id: UserId,
        member_id: HouseholdMemberId,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        if entry.scope != MealPlanScope::Household {
            return Err(CoreError::conflict(
                "You can only opt out of a household meal.",
            ));
        }
        let Some(participant) = entry.participant_for(member_id) else {
            if entry.has_opted_out(member_id) {
                return self.get(id).await;
            }
            return Err(CoreError::conflict("You are not part of this meal."));
        };
        if participant
            .allocations
            .iter()
            .any(|allocation| allocation.status.is_resolved())
        {
            return Err(CoreError::conflict(
                "Reopen your portion in the food log before opting out.",
            ));
        }

        let now = self.clock.now();
        entry
            .participants
            .retain(|participant| participant.member_id != member_id);
        entry.opted_out.push(MealOptOut {
            member_id,
            created_by: actor_id,
            created_at: now,
        });
        entry.updated_by = actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        apply_equal_portioning(&mut entry);
        commit_outcome(
            self.plans.set_participants(&entry, expected).await?,
            id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn opt_in(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor_id: UserId,
        member_id: HouseholdMemberId,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        if entry.scope != MealPlanScope::Household {
            return Err(CoreError::conflict(
                "You can only opt in to a household meal.",
            ));
        }
        if entry.participant_for(member_id).is_some() {
            return self.get(id).await;
        }
        let member = self
            .members
            .get(member_id)
            .await?
            .ok_or_else(|| CoreError::not_found("household member", member_id))?;
        if member.is_archived() {
            return Err(CoreError::conflict("Archived members cannot join a meal."));
        }
        self.ensure_slot_free(
            member_id,
            entry.planned_on,
            entry.slot,
            entry.planned_time,
            Some(entry.id),
        )
        .await?;

        let now = self.clock.now();
        entry
            .opted_out
            .retain(|opt_out| opt_out.member_id != member_id);
        entry
            .participants
            .push(build_participant(member_id, &entry.components, now, false));
        entry.updated_by = actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        apply_equal_portioning(&mut entry);
        commit_outcome(
            self.plans.set_participants(&entry, expected).await?,
            id,
            expected,
        )?;
        self.get(id).await
    }

    pub async fn slot_attendance(
        &self,
        planned_on: Date,
        slot: MealSlot,
        exclude_entry: Option<MealPlanEntryId>,
    ) -> Result<Vec<(HouseholdMemberId, SlotAttendance, Option<Time>)>> {
        let members = self.active_member_ids().await?;
        let day = self.plans.list_all(planned_on, planned_on).await?;
        let mut result = Vec::with_capacity(members.len());
        for member_id in members {
            let mut attendance = SlotAttendance::Available;
            let mut claimed_time = None;
            for entry in &day {
                if entry.slot != slot || Some(entry.id) == exclude_entry {
                    continue;
                }
                match entry.scope {
                    MealPlanScope::Member if entry.member_id == Some(member_id) => {
                        attendance = SlotAttendance::SelfCatering;
                        claimed_time = entry.planned_time;
                    }
                    MealPlanScope::Household if entry.participant_for(member_id).is_some() => {
                        attendance = SlotAttendance::Participating;
                        claimed_time = entry.planned_time;
                    }
                    MealPlanScope::Household
                        if entry.has_opted_out(member_id)
                            && attendance == SlotAttendance::Available =>
                    {
                        attendance = SlotAttendance::OptedOut;
                    }
                    _ => {}
                }
            }
            result.push((member_id, attendance, claimed_time));
        }
        Ok(result)
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
        planned_time: Option<Time>,
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
            .any(|entry| {
                entry.slot == slot
                    && (slot != MealSlot::Snacks || entry.planned_time == planned_time)
                    && Some(entry.id) != ignore_entry
                    && !(entry.has_opted_out(member_id)
                        && entry.participant_for(member_id).is_none())
            });
        if clash {
            let message = if slot == MealSlot::Snacks {
                if planned_time.is_some() {
                    "A snack is already planned for that time. Edit it to add more food."
                } else {
                    "An untimed snack is already planned. Edit it to add more food."
                }
            } else {
                "That meal slot already exists. Add food to the existing meal instead."
            };
            return Err(CoreError::conflict(message));
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
        require_editable(&entry)?;

        let now = self.clock.now();
        if let Some(portioning) = patch.portioning {
            entry.portioning = portioning;
        }
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
        if let Some(participants) = patch.participants {
            validate_participants(&participants, &entry.components)?;
            if has_explicit_allocations(&participants) {
                entry.portioning = Portioning::Custom;
            }
            entry.participants = participants
                .iter()
                .map(|participant| {
                    let previous = entry
                        .participants
                        .iter()
                        .find(|candidate| candidate.member_id == participant.member_id);
                    merge_participant(previous, participant, &entry.components, now)
                })
                .collect();
        }
        if let Some(guest_groups) = patch.guest_groups {
            validate_guest_groups(&guest_groups, &entry.components)?;
            entry.guest_groups = guest_groups
                .iter()
                .map(|group| merge_guest_group(&entry.guest_groups, group, now))
                .collect();
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
        if entry.scope == MealPlanScope::Household && entry.slot == MealSlot::Snacks {
            return Err(CoreError::conflict(
                "Snacks stay on your own planner. Household planning covers breakfast, lunch and dinner.",
            ));
        }
        if entry.scope == MealPlanScope::Member {
            let owner = entry.member_id.expect("personal meal owner");
            if !entry.guest_groups.is_empty()
                || entry.participants.len() != 1
                || entry.participants[0].member_id != owner
            {
                return Err(CoreError::conflict(
                    "A personal meal can only contain its owner.",
                ));
            }
        }
        require_household_attendance(entry.scope, &entry.participants, &entry.guest_groups)?;
        for participant in &entry.participants {
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
            self.ensure_slot_free(
                participant.member_id,
                entry.planned_on,
                entry.slot,
                entry.planned_time,
                Some(entry.id),
            )
            .await?;
        }
        entry.updated_by = actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        apply_equal_portioning(&mut entry);
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
    ) -> Result<StockAffected<MealPlanEntryView>> {
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
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, input.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_component_revision(component_id, expected, component.revision)?;
        let component_item = component.item;
        let planned_amount = component.amount;
        let old_component_revision = component.revision;

        require_allocation_planned(&entry, subject, component_id)?;
        let already_drawn = component_still_eaten(&entry, component_id);

        let catalogue = self.catalogue_for([component_item]).await?;
        let planned = catalogue.resolve(component_item, &planned_amount);
        let actual = catalogue.resolve(component_item, &input.amount);
        if !actual.resolvable {
            let mut errors = ValidationErrors::new();
            errors.push("amount", "We cannot work out this item's nutrition");
            return Err(errors.into());
        }
        let write = StockWrite {
            deductions: if already_drawn {
                Vec::new()
            } else {
                self.component_deduction(
                    &catalogue,
                    &entry,
                    component_id,
                    component_item,
                    &planned_amount,
                    input.actor_id,
                    Some(subject),
                )
            },
            releases: Vec::new(),
        };
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
        apply_equal_portioning(&mut entry);
        let update = component_update(
            &entry,
            component_id,
            SnapshotOp::Set(&snapshot),
            old_component_revision,
            input.actor_id,
            now,
        );
        let (outcome, stock_outcomes) = self
            .plans
            .resolve_component(
                id,
                &update,
                &entry.participants,
                expected,
                Some(&record),
                &write,
            )
            .await?;
        commit_component_outcome(outcome, component_id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn mark_component_not_eaten(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
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
    ) -> Result<StockAffected<MealPlanEntryView>> {
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
        apply_equal_portioning(&mut entry);
        let update = component_update(
            &entry,
            component_id,
            SnapshotOp::Set(&snapshot),
            old_component_revision,
            actor.actor_id,
            now,
        );
        let (outcome, stock_outcomes) = self
            .plans
            .resolve_component(
                id,
                &update,
                &entry.participants,
                expected,
                None,
                &StockWrite::default(),
            )
            .await?;
        commit_component_outcome(outcome, component_id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn reopen_component(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, actor.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_component_revision(component_id, expected, component.revision)?;
        let old_component_revision = component.revision;
        let component_item = component.item;

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

        let catalogue = self.catalogue_for([component_item]).await?;
        let now = self.clock.now();
        set_allocation(
            &mut entry,
            subject,
            component_id,
            ParticipantStatus::Planned,
            None,
            None,
            None,
        );
        apply_equal_portioning(&mut entry);
        let still_eaten = component_still_eaten(&entry, component_id);
        let snapshot_op = if still_eaten {
            SnapshotOp::Keep
        } else {
            SnapshotOp::Clear
        };
        let write = StockWrite {
            deductions: Vec::new(),
            releases: if still_eaten {
                Vec::new()
            } else {
                vec![self.component_release(
                    &catalogue,
                    &entry,
                    component_id,
                    component_item,
                    actor.actor_id,
                    subject,
                )]
            },
        };
        let update = component_update(
            &entry,
            component_id,
            snapshot_op,
            old_component_revision,
            actor.actor_id,
            now,
        );
        let (outcome, stock_outcomes) = self
            .plans
            .reopen_component(
                id,
                &update,
                &entry.participants,
                expected,
                record_to_remove,
                &write,
            )
            .await?;
        commit_component_outcome(outcome, component_id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn mark_not_eaten(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_not_eaten_unchecked(id, expected, actor).await
    }

    pub async fn mark_not_eaten_unchecked(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
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
        apply_equal_portioning(&mut entry);
        let (outcome, stock_outcomes) = self
            .plans
            .resolve(&entry, expected, &[], &StockWrite::default())
            .await?;
        commit_outcome(outcome, id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn mark_eaten(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ConfirmMealPlanEntry,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_eaten_unchecked(id, expected, input).await
    }

    pub async fn mark_eaten_unchecked(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ConfirmMealPlanEntry,
    ) -> Result<StockAffected<MealPlanEntryView>> {
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
        let mut deductions: Vec<StockDeduction> = Vec::new();
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
            if !component_still_eaten(&entry, *component_id) {
                deductions.extend(self.component_deduction(
                    &catalogue,
                    &entry,
                    *component_id,
                    component.item,
                    &component.amount,
                    input.actor_id,
                    Some(subject),
                ));
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
        apply_equal_portioning(&mut entry);
        let write = StockWrite {
            deductions,
            releases: Vec::new(),
        };
        let (outcome, stock_outcomes) = self
            .plans
            .resolve(&entry, expected, &records, &write)
            .await?;
        commit_outcome(outcome, id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn review_outcomes(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ReviewMealOutcomes,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.review_outcomes_unchecked(id, expected, input).await
    }

    pub async fn review_outcomes_unchecked(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ReviewMealOutcomes,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        if input.members.is_empty() && input.guests.is_empty() {
            return Err(CoreError::conflict("Choose at least one person."));
        }
        self.freeze(&mut entry).await?;
        let replacement_items: Vec<MealItemRef> = input
            .members
            .iter()
            .flat_map(|reviewed| replacements_for(&reviewed.outcome))
            .map(|replacement| replacement.item)
            .collect();
        self.validate_replacement_items(&replacement_items).await?;
        let catalogue = self
            .catalogue_for(
                entry
                    .components
                    .iter()
                    .map(|component| component.item)
                    .chain(replacement_items.iter().copied()),
            )
            .await?;
        let now = self.clock.now();
        let mut records = Vec::new();
        let mut deductions = Vec::new();

        for reviewed in &input.members {
            require_subject_pending(&entry, reviewed.member_id)?;
            let pending = pending_component_ids(&entry, reviewed.member_id);
            let actual = actual_components_for_member(
                &reviewed.outcome,
                &entry,
                reviewed.member_id,
                &pending,
            )?;
            for component_id in pending {
                let component = find_component(&entry, component_id)?.clone();
                if let Some(amount) = actual.get(&component_id).copied() {
                    let scaled = catalogue.resolve(component.item, &amount);
                    if !scaled.resolvable {
                        let mut errors = ValidationErrors::new();
                        errors.push(
                            format!("members.{}.components.{component_id}", reviewed.member_id),
                            "We cannot work out this food's nutrition",
                        );
                        return Err(errors.into());
                    }
                    if !component_still_eaten(&entry, component_id) {
                        deductions.extend(self.component_deduction(
                            &catalogue,
                            &entry,
                            component_id,
                            component.item,
                            &component.amount,
                            input.actor_id,
                            Some(reviewed.member_id),
                        ));
                    }
                    let record = ConsumptionRecord {
                        id: Default::default(),
                        member_id: reviewed.member_id,
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
                        reviewed.member_id,
                        component_id,
                        ParticipantStatus::Eaten,
                        Some(record.id),
                        Some(input.actor_id),
                        Some(now),
                    );
                    records.push(record);
                } else {
                    set_allocation(
                        &mut entry,
                        reviewed.member_id,
                        component_id,
                        ParticipantStatus::NotEaten,
                        None,
                        Some(input.actor_id),
                        Some(now),
                    );
                }
            }

            for replacement in replacements_for(&reviewed.outcome) {
                let scaled = catalogue.resolve(replacement.item, &replacement.amount);
                if !scaled.resolvable {
                    let mut errors = ValidationErrors::new();
                    errors.push(
                        format!("members.{}.replacements", reviewed.member_id),
                        "We cannot work out this food's nutrition",
                    );
                    return Err(errors.into());
                }
                let record = ConsumptionRecord {
                    id: Default::default(),
                    member_id: reviewed.member_id,
                    item: replacement.item,
                    recorded_by: Some(input.actor_id),
                    meal_plan_entry_id: Some(entry.id),
                    meal_plan_component_id: None,
                    slot: entry.slot,
                    amount: replacement.amount,
                    consumed_on: input.consumed_on,
                    consumed_at: input.consumed_at,
                    nutrition: scaled.nutrition.facts,
                    quality: scaled.nutrition.quality,
                    revision: Revision::INITIAL,
                    created_at: now,
                    updated_at: now,
                };
                deductions.extend(self.record_deduction_for(&catalogue, &record));
                records.push(record);
            }
        }

        let guest_results = build_guest_results(&entry, &input, now)?;
        let mut guest_deductions = HashSet::new();
        for group in &guest_results {
            for allocation in &group.allocations {
                if allocation.status == ParticipantStatus::Eaten
                    && !component_still_eaten(&entry, allocation.component_id)
                    && guest_deductions.insert(allocation.component_id)
                {
                    let component = find_component(&entry, allocation.component_id)?.clone();
                    deductions.extend(self.component_deduction(
                        &catalogue,
                        &entry,
                        component.id,
                        component.item,
                        &component.amount,
                        input.actor_id,
                        None,
                    ));
                }
            }
        }
        if !input.guests.is_empty() {
            let source_ids: HashSet<_> = input
                .guests
                .iter()
                .map(|result| result.source_group_id)
                .collect();
            entry
                .guest_groups
                .retain(|group| !source_ids.contains(&group.id));
            entry.guest_groups.extend(guest_results);
        }

        entry.updated_by = input.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        apply_equal_portioning(&mut entry);
        let write = StockWrite {
            deductions,
            releases: Vec::new(),
        };
        let (outcome, stock_outcomes) = self
            .plans
            .resolve(&entry, expected, &records, &write)
            .await?;
        commit_outcome(outcome, id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn reopen(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
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
        let catalogue = self
            .catalogue_for(entry.components.iter().map(|component| component.item))
            .await?;
        let now = self.clock.now();
        for &component_id in &resolved {
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
                && derive_component_status(
                    component.id,
                    &entry.participants,
                    &entry.guest_groups,
                    Assumption::NONE,
                ) == MealPlanStatus::Planned
            {
                component.snapshot = None;
            }
        }
        let mut releases: Vec<StockRelease> = Vec::new();
        for &component_id in &resolved {
            if !component_still_eaten(&entry, component_id) {
                let item = entry
                    .components
                    .iter()
                    .find(|c| c.id == component_id)
                    .map(|c| c.item);
                if let Some(item) = item {
                    releases.push(self.component_release(
                        &catalogue,
                        &entry,
                        component_id,
                        item,
                        actor.actor_id,
                        subject,
                    ));
                }
            }
        }
        entry.updated_by = actor.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        apply_equal_portioning(&mut entry);
        let write = StockWrite {
            deductions: Vec::new(),
            releases,
        };
        let (outcome, stock_outcomes) = self
            .plans
            .reopen(&entry, expected, &record_ids, &write)
            .await?;
        commit_outcome(outcome, id, expected)?;
        self.stock_affected(id, stock_outcomes).await
    }

    async fn stock_affected(
        &self,
        id: MealPlanEntryId,
        outcomes: Vec<StockOutcome>,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let view = self.get(id).await?;
        let named = name_outcomes(&*self.products, &*self.ingredients, outcomes).await?;
        Ok(StockAffected::new(view, named))
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn component_deduction(
        &self,
        catalogue: &ItemCatalogue,
        entry: &MealPlanEntry,
        component_id: MealPlanComponentId,
        item: MealItemRef,
        amount: &ConsumedAmount,
        actor: UserId,
        subject: Option<HouseholdMemberId>,
    ) -> Vec<StockDeduction> {
        match item {
            MealItemRef::Product { product_id } => {
                let Some(product) = catalogue.products.get(&product_id) else {
                    return Vec::new();
                };
                product_deduction(
                    StockEffectSource::MealPlanComponent,
                    component_id.as_uuid(),
                    product,
                    amount,
                    stock_source_label(entry, &product.name),
                    Some(actor),
                    subject,
                )
                .into_iter()
                .collect()
            }
            MealItemRef::Recipe { recipe_id } => {
                let label = catalogue.name_of(item);
                catalogue
                    .recipe_wants(recipe_id, amount)
                    .into_iter()
                    .map(|want| {
                        requirement_deduction(
                            StockEffectSource::MealPlanComponent,
                            component_id.as_uuid(),
                            want.recipe_component_id.as_uuid(),
                            want.target,
                            want.want,
                            stock_source_label(entry, &label),
                            Some(actor),
                            subject,
                        )
                    })
                    .collect()
            }
        }
    }

    fn record_deduction_for(
        &self,
        catalogue: &ItemCatalogue,
        record: &ConsumptionRecord,
    ) -> Vec<StockDeduction> {
        match record.item {
            MealItemRef::Product { product_id } => {
                let Some(product) = catalogue.products.get(&product_id) else {
                    return Vec::new();
                };
                let label = product.name.clone();
                record_deduction(record, product, label)
                    .into_iter()
                    .collect()
            }
            MealItemRef::Recipe { recipe_id } => {
                let label = catalogue.name_of(record.item);
                catalogue
                    .recipe_wants(recipe_id, &record.amount)
                    .into_iter()
                    .map(|want| {
                        requirement_deduction(
                            StockEffectSource::ConsumptionRecord,
                            record.id.as_uuid(),
                            want.recipe_component_id.as_uuid(),
                            want.target,
                            want.want,
                            label.clone(),
                            record.recorded_by,
                            Some(record.member_id),
                        )
                    })
                    .collect()
            }
        }
    }

    async fn validate_replacement_items(&self, items: &[MealItemRef]) -> Result<()> {
        for item in items {
            match *item {
                MealItemRef::Product { product_id } => {
                    self.products
                        .get(product_id)
                        .await?
                        .ok_or_else(|| CoreError::not_found(PRODUCT, product_id))?;
                }
                MealItemRef::Recipe { recipe_id } => {
                    self.recipes
                        .get(recipe_id)
                        .await?
                        .ok_or_else(|| CoreError::not_found(RECIPE, recipe_id))?;
                }
            }
        }
        Ok(())
    }

    fn component_release(
        &self,
        catalogue: &ItemCatalogue,
        entry: &MealPlanEntry,
        component_id: MealPlanComponentId,
        item: MealItemRef,
        actor: UserId,
        subject: HouseholdMemberId,
    ) -> StockRelease {
        let name = match item {
            MealItemRef::Product { product_id } => catalogue
                .products
                .get(&product_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "food".to_owned()),
            MealItemRef::Recipe { .. } => "food".to_owned(),
        };
        component_release(
            component_id.as_uuid(),
            stock_source_label(entry, &name),
            Some(actor),
            Some(subject),
        )
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

        let rules = self.assumption_rules().await?;
        let mut presented_by_date: BTreeMap<Date, Vec<MealPlanEntryView>> = BTreeMap::new();
        let mut items_by_slot: HashMap<(Date, MealSlot), Vec<(MealItemOrder, MealItem)>> =
            HashMap::new();
        for entry in entries {
            let linked = records_by_entry
                .get(&entry.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let date = entry.planned_on;
            let view = self
                .present_with(&rules, entry, linked, Some(member_id))
                .await?;
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

    pub async fn planner_entries(&self, week_start: Date) -> Result<Vec<MealPlanEntryView>> {
        let week_end = week_start + Duration::days(6);
        let entries = self.plans.list_all(week_start, week_end).await?;
        let rules = self.assumption_rules().await?;
        let mut views = Vec::with_capacity(entries.len());
        for entry in entries {
            let records = self.records_for_entry(entry.id).await?;
            views.push(self.present_with(&rules, entry, &records, None).await?);
        }
        Ok(views)
    }

    pub async fn needs_review(
        &self,
        member_id: HouseholdMemberId,
        include_household: bool,
    ) -> Result<NeedsReview> {
        let rules = self.assumption_rules().await?;
        let today = rules.now.date();
        let mut personal = Vec::new();
        for entry in self.plans.list_through(member_id, today).await? {
            let records = self.records_for_entry(entry.id).await?;
            let view = self
                .present_with(&rules, entry, &records, Some(member_id))
                .await?;
            if view.subject_status == MealPlanStatus::Assumed {
                personal.push(view);
            }
        }

        let mut household = Vec::new();
        if include_household {
            for entry in self
                .plans
                .list_all_through(today)
                .await?
                .into_iter()
                .filter(|entry| entry.scope == MealPlanScope::Household)
            {
                let records = self.records_for_entry(entry.id).await?;
                let view = self.present_with(&rules, entry, &records, None).await?;
                if view.status == MealPlanStatus::Assumed
                    || (view.status == MealPlanStatus::PartiallyResolved && view.assumption.assumed)
                {
                    household.push(view);
                }
            }
        }

        personal.sort_by_key(|view| (view.entry.planned_on, view.entry.slot.order()));
        household.sort_by_key(|view| (view.entry.planned_on, view.entry.slot.order()));
        Ok(NeedsReview {
            personal,
            household,
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
        requested
            .or(entry.member_id)
            .or_else(|| entry.participants.first().map(|p| p.member_id))
            .ok_or_else(|| {
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
                        .filter(|recipe| {
                            recipe.owner_id == actor_id
                                || recipe.visibility == RecipeVisibility::Shared
                        })
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
        let cards: HashMap<RecipeId, RecipeCard> = recipes
            .iter()
            .map(|recipe| (recipe.id, recipe_card(recipe, &fulfilments)))
            .collect();
        let definitions: HashMap<RecipeId, Recipe> = recipes
            .into_iter()
            .map(|recipe| (recipe.id, recipe))
            .collect();

        Ok(ItemCatalogue {
            products,
            recipes: cards,
            definitions,
            fulfilments,
        })
    }

    async fn present(
        &self,
        entry: MealPlanEntry,
        records: &[ConsumptionRecord],
        requested_subject: Option<HouseholdMemberId>,
    ) -> Result<MealPlanEntryView> {
        let rules = self.assumption_rules().await?;
        self.present_with(&rules, entry, records, requested_subject)
            .await
    }

    async fn present_with(
        &self,
        rules: &AssumptionRules,
        entry: MealPlanEntry,
        records: &[ConsumptionRecord],
        requested_subject: Option<HouseholdMemberId>,
    ) -> Result<MealPlanEntryView> {
        let subject = requested_subject
            .or(entry.member_id)
            .or_else(|| entry.participants.first().map(|p| p.member_id));
        let assumption = rules.for_entry(&entry);
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
                        .map(|a| participant_status_to_meal(a.status, assumption))
                })
                .unwrap_or_else(|| entry.component_status(component.id, assumption));

            components.push(MealPlanComponentView {
                component: component.clone(),
                item_name,
                nutrition,
                quality,
                consumption_record: subject_record,
                preparation,
                status: entry.component_status(component.id, assumption),
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
                status: derive_participant_status(participant, assumption),
                allocations: participant.allocations.clone(),
                nutrition: summary(participant_records.into_iter()),
            });
        }

        let planned = summary_components(
            components
                .iter()
                .filter(|component| component.subject_status.is_unresolved()),
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

        let status = entry.status(assumption);
        let subject_status = subject
            .and_then(|member| entry.participant_for(member))
            .map(|participant| derive_participant_status(participant, assumption))
            .unwrap_or(status);

        Ok(MealPlanEntryView {
            entry,
            subject_member_id: subject,
            components,
            participants: participant_views,
            planned,
            actual,
            needs_attention,
            assumption,
            status,
            subject_status,
        })
    }

    async fn assumption_rules(&self) -> Result<AssumptionRules> {
        let settings = self.settings.get().await?;
        Ok(AssumptionRules {
            now: self.clock.now(),
            meal_times: settings.meal_times,
            enabled: settings.assume_eaten_when_time_passes,
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

fn validate_guest_groups(
    groups: &[NewMealGuestGroup],
    components: &[MealPlanComponent],
) -> Result<()> {
    let mut errors = ValidationErrors::new();
    for (group_index, group) in groups.iter().enumerate() {
        if group.count <= 0 {
            errors.push(
                format!("guest_groups.{group_index}.count"),
                "Guest count must be more than zero",
            );
        }
        let mut seen = HashSet::new();
        for (allocation_index, allocation) in group.allocations.iter().enumerate() {
            let field = format!("guest_groups.{group_index}.allocations.{allocation_index}");
            if !seen.insert(allocation.component_id) {
                errors.push(&field, "This food appears twice");
                continue;
            }
            let Some(component) = components
                .iter()
                .find(|component| component.id == allocation.component_id)
            else {
                errors.push(&field, "Unknown food");
                continue;
            };
            if allocation.allocated.value() <= Decimal::ZERO {
                errors.push(format!("{field}.amount"), "Must be more than zero");
            }
            if allocation.allocated.kind_code() != component.amount.kind_code() {
                errors.push(
                    format!("{field}.amount"),
                    "Portion must use the meal amount type",
                );
            }
        }
    }
    errors.into_result()
}

fn merge_guest_group(
    existing: &[MealGuestGroup],
    new_group: &NewMealGuestGroup,
    now: OffsetDateTime,
) -> MealGuestGroup {
    let previous = new_group
        .id
        .and_then(|id| existing.iter().find(|group| group.id == id));
    let allocations = new_group
        .allocations
        .iter()
        .map(|allocation| {
            let old = previous.and_then(|group| {
                group
                    .allocations
                    .iter()
                    .find(|candidate| candidate.component_id == allocation.component_id)
            });
            MealGuestAllocation {
                id: old.map(|value| value.id).unwrap_or_default(),
                component_id: allocation.component_id,
                allocated: allocation.allocated,
                status: old
                    .map(|value| value.status)
                    .unwrap_or(ParticipantStatus::Planned),
                confirmed: old.and_then(|value| value.confirmed),
                resolved_by: old.and_then(|value| value.resolved_by),
                resolved_at: old.and_then(|value| value.resolved_at),
            }
        })
        .collect();
    MealGuestGroup {
        id: previous
            .map(|group| group.id)
            .or(new_group.id)
            .unwrap_or_default(),
        count: new_group.count,
        allocations,
        revision: previous
            .map(|group| group.revision.next())
            .unwrap_or(Revision::INITIAL),
        created_at: previous.map(|group| group.created_at).unwrap_or(now),
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

fn participant_status_to_meal(status: ParticipantStatus, assumption: Assumption) -> MealPlanStatus {
    match status {
        ParticipantStatus::Planned => {
            if assumption.assumed {
                MealPlanStatus::Assumed
            } else {
                MealPlanStatus::Planned
            }
        }
        ParticipantStatus::Eaten => MealPlanStatus::Eaten,
        ParticipantStatus::NotEaten => MealPlanStatus::NotEaten,
    }
}

fn has_explicit_allocations(participants: &[NewMealParticipant]) -> bool {
    participants
        .iter()
        .any(|participant| !participant.allocations.is_empty())
}

fn apply_equal_portioning(entry: &mut MealPlanEntry) {
    if entry.portioning != Portioning::Equal {
        return;
    }
    let guest_heads: usize = entry
        .guest_groups
        .iter()
        .map(|group| group.count.max(0) as usize)
        .sum();
    let shares = entry.participants.len() + guest_heads;
    if shares == 0 {
        return;
    }
    let components = entry.components.clone();
    for participant in &mut entry.participants {
        for allocation in &mut participant.allocations {
            if allocation.status == ParticipantStatus::Planned
                && let Some(component) = components
                    .iter()
                    .find(|component| component.id == allocation.component_id)
            {
                allocation.allocated = equal_split(&component.amount, shares);
            }
        }
    }
    for group in &mut entry.guest_groups {
        for allocation in &mut group.allocations {
            if allocation.status == ParticipantStatus::Planned
                && let Some(component) = components
                    .iter()
                    .find(|component| component.id == allocation.component_id)
            {
                allocation.allocated = equal_split(&component.amount, shares);
            }
        }
    }
}

fn require_household_attendance<P, G>(
    scope: MealPlanScope,
    participants: &[P],
    guest_groups: &[G],
) -> Result<()> {
    if scope == MealPlanScope::Household && participants.is_empty() && guest_groups.is_empty() {
        return Err(CoreError::conflict(
            "A household meal needs at least one household member or guest.",
        ));
    }
    Ok(())
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
    let mut outcomes: Vec<AllocationOutcome> = entry
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
        .collect();
    for group in &entry.guest_groups {
        if let Some(allocation) = group
            .allocations
            .iter()
            .find(|allocation| allocation.component_id == component_id)
        {
            for _ in 0..group.count {
                outcomes.push(AllocationOutcome {
                    allocated: allocation.allocated,
                    status: allocation.status,
                    confirmed: allocation.confirmed,
                });
            }
        }
    }
    outcomes
}

fn actual_components_for_member(
    outcome: &ReviewedMealOutcome,
    entry: &MealPlanEntry,
    member_id: HouseholdMemberId,
    pending: &[MealPlanComponentId],
) -> Result<HashMap<MealPlanComponentId, ConsumedAmount>> {
    match outcome {
        ReviewedMealOutcome::NotEaten => Ok(HashMap::new()),
        ReviewedMealOutcome::AsPlanned => Ok(entry
            .participant_for(member_id)
            .into_iter()
            .flat_map(|participant| participant.allocations.iter())
            .filter(|allocation| pending.contains(&allocation.component_id))
            .map(|allocation| (allocation.component_id, allocation.allocated))
            .collect()),
        ReviewedMealOutcome::Changed(changed) => {
            if changed.is_empty() {
                return Err(CoreError::conflict(
                    "Choose what was eaten, or record the meal as not eaten.",
                ));
            }
            validate_reviewed_components(&changed.components, entry, pending)
        }
    }
}

fn replacements_for(outcome: &ReviewedMealOutcome) -> &[ReplacementItem] {
    match outcome {
        ReviewedMealOutcome::Changed(changed) => &changed.replacements,
        _ => &[],
    }
}

fn validate_reviewed_components(
    components: &[crate::domain::ActualMealPlanComponent],
    entry: &MealPlanEntry,
    pending: &[MealPlanComponentId],
) -> Result<HashMap<MealPlanComponentId, ConsumedAmount>> {
    let mut errors = ValidationErrors::new();
    let mut actual = HashMap::new();
    for component in components {
        let Some(planned) = entry
            .components
            .iter()
            .find(|candidate| candidate.id == component.component_id)
        else {
            errors.push("components", "Unknown planned food");
            continue;
        };
        if !pending.contains(&component.component_id) {
            errors.push("components", "This result has already been recorded");
        }
        if component.amount.value() <= Decimal::ZERO {
            errors.push("components", "Amounts must be more than zero");
        }
        if component.amount.kind_code() != planned.amount.kind_code() {
            errors.push("components", "Amount type must match the planned food");
        }
        if actual
            .insert(component.component_id, component.amount)
            .is_some()
        {
            errors.push("components", "This food appears twice");
        }
    }
    errors.into_result()?;
    Ok(actual)
}

fn build_guest_results(
    entry: &MealPlanEntry,
    input: &ReviewMealOutcomes,
    now: OffsetDateTime,
) -> Result<Vec<MealGuestGroup>> {
    let mut results = Vec::new();
    let source_ids: HashSet<_> = input
        .guests
        .iter()
        .map(|reviewed| reviewed.source_group_id)
        .collect();
    for source_id in source_ids {
        let source = entry
            .guest_groups
            .iter()
            .find(|group| group.id == source_id)
            .ok_or_else(|| CoreError::conflict("These guests are no longer part of the meal."))?;
        if source
            .allocations
            .iter()
            .any(|allocation| allocation.status.is_resolved())
        {
            return Err(CoreError::conflict(
                "A guest result has already been recorded.",
            ));
        }
        let reviewed: Vec<_> = input
            .guests
            .iter()
            .filter(|candidate| candidate.source_group_id == source_id)
            .collect();
        if reviewed.iter().any(|candidate| candidate.count <= 0)
            || reviewed
                .iter()
                .map(|candidate| candidate.count)
                .sum::<i32>()
                != source.count
        {
            let mut errors = ValidationErrors::new();
            errors.push(
                "guests",
                "Guest results must add up to the planned guest count",
            );
            return Err(errors.into());
        }
        let pending: Vec<_> = source
            .allocations
            .iter()
            .map(|allocation| allocation.component_id)
            .collect();
        for reviewed in reviewed {
            let actual = match &reviewed.outcome {
                ReviewedMealOutcome::AsPlanned => source
                    .allocations
                    .iter()
                    .map(|allocation| (allocation.component_id, allocation.allocated))
                    .collect(),
                ReviewedMealOutcome::NotEaten => HashMap::new(),
                ReviewedMealOutcome::Changed(changed) => {
                    if !changed.replacements.is_empty() {
                        return Err(CoreError::conflict(
                            "Guests cannot have different food recorded against them.",
                        ));
                    }
                    validate_reviewed_components(&changed.components, entry, &pending)?
                }
            };
            results.push(MealGuestGroup {
                id: Default::default(),
                count: reviewed.count,
                allocations: source
                    .allocations
                    .iter()
                    .map(|allocation| {
                        let confirmed = actual.get(&allocation.component_id).copied();
                        MealGuestAllocation {
                            id: Default::default(),
                            component_id: allocation.component_id,
                            allocated: allocation.allocated,
                            status: if confirmed.is_some() {
                                ParticipantStatus::Eaten
                            } else {
                                ParticipantStatus::NotEaten
                            },
                            confirmed,
                            resolved_by: Some(input.actor_id),
                            resolved_at: Some(now),
                        }
                    })
                    .collect(),
                revision: Revision::INITIAL,
                created_at: source.created_at,
                updated_at: now,
            });
        }
    }
    Ok(results)
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
    _entry: &MealPlanEntry,
    component_id: MealPlanComponentId,
    snapshot: SnapshotOp<'a>,
    old_revision: Revision,
    actor_id: UserId,
    now: OffsetDateTime,
) -> MealPlanComponentUpdate<'a> {
    MealPlanComponentUpdate {
        id: component_id,
        snapshot,
        revision: old_revision.next(),
        actor_id,
        now,
    }
}

fn component_still_eaten(entry: &MealPlanEntry, component_id: MealPlanComponentId) -> bool {
    entry.participants.iter().any(|participant| {
        participant.allocations.iter().any(|allocation| {
            allocation.component_id == component_id && allocation.status == ParticipantStatus::Eaten
        })
    }) || entry.guest_groups.iter().any(|group| {
        group.allocations.iter().any(|allocation| {
            allocation.component_id == component_id && allocation.status == ParticipantStatus::Eaten
        })
    })
}

fn stock_source_label(entry: &MealPlanEntry, item_name: &str) -> String {
    let slot = match entry.slot {
        MealSlot::Breakfast => "Breakfast",
        MealSlot::Lunch => "Lunch",
        MealSlot::Dinner => "Dinner",
        MealSlot::Snacks => "Snacks",
    };
    format!("{slot} {} \u{2014} {item_name}", entry.planned_on)
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
    if entry.status(Assumption::NONE) == MealPlanStatus::Planned {
        Ok(())
    } else {
        Err(CoreError::conflict(
            "Resolved meal plans cannot be changed.",
        ))
    }
}

fn require_editable(entry: &MealPlanEntry) -> Result<()> {
    match entry.status(Assumption::NONE) {
        MealPlanStatus::Planned | MealPlanStatus::Assumed | MealPlanStatus::PartiallyResolved => {
            Ok(())
        }
        MealPlanStatus::Eaten | MealPlanStatus::NotEaten => Err(CoreError::conflict(
            "This meal is fully resolved and cannot be changed.",
        )),
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
        .filter(|component| component.subject_status.is_unresolved())
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
