use std::collections::{HashMap, HashSet};

use time::OffsetDateTime;

use crate::domain::{
    Assumption, ConfirmMealPlanComponent, ConfirmMealPlanEntry, ConsumedAmount, ConsumptionRecord,
    ConsumptionRecordId, HouseholdMemberId, MEAL_PLAN_COMPONENT, MEAL_PLAN_ENTRY, MealItemRef,
    MealPlanComponentId, MealPlanComponentSnapshot, MealPlanEntry, MealPlanEntryId, MealPlanStatus,
    MealSlot, NewConsumptionRecord, OutcomeActor, ParticipantStatus, PreparedBatch, Quantity,
    ReviewMealOutcomes, Revision, StockEffectSource, StockOutcome, Unit, UserId,
    actual_components_for_member, build_guest_results, component_still_eaten,
    derive_component_status, find_component, pending_component_ids, replacements_for,
    require_allocation_planned, require_subject_pending, set_allocation,
    validate_actual_components,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{MealPlanComponentUpdate, SnapshotOp, StockDeduction, StockRelease, StockWrite};

use super::catalogue::ItemCatalogue;
use super::view::MealPlanEntryView;
use super::{DISH, MealPlanService, PRODUCT, RECIPE, ensure_due};
use crate::services::revision::{commit_outcome, require_revision};
use crate::services::stock_effects::{
    StockAffected, component_release, cooked_food_deduction, name_outcomes, portion_deduction,
    product_deduction, record_deduction, requirement_deduction,
};

impl MealPlanService {
    pub async fn mark_component_eaten(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        input: ConfirmMealPlanComponent,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let entry = self.get_entry(id).await?;
        ensure_due(&*self.clock, entry.planned_on)?;
        self.mark_component_eaten_backdated(id, component_id, expected, input)
            .await
    }

    pub async fn mark_component_eaten_backdated(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        input: ConfirmMealPlanComponent,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, input.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_revision(
            MEAL_PLAN_COMPONENT,
            component_id,
            expected,
            component.revision,
        )?;
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
            deductions: self
                .component_deduction(
                    &catalogue,
                    &entry,
                    component_id,
                    component_item,
                    &planned_amount,
                    &input.amount,
                    subject.as_uuid(),
                    already_drawn,
                    input.actor_id,
                    Some(subject),
                )
                .await?,
            releases: Vec::new(),
        };
        let now = self.clock.now();
        let record = ConsumptionRecord::create(
            NewConsumptionRecord {
                id: None,
                member_id: subject,
                item: component_item,
                recorded_by: Some(input.actor_id),
                meal_plan_entry_id: Some(entry.id),
                meal_plan_component_id: Some(component_id),
                slot: entry.slot,
                amount: input.amount,
                consumed_on: input.consumed_on,
                consumed_at: input.consumed_at,
            },
            actual.nutrition.facts,
            actual.nutrition.quality,
            now,
        );

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
        let update = component_update(
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
        commit_outcome(MEAL_PLAN_COMPONENT, component_id, expected, outcome)?;
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
        self.mark_component_not_eaten_backdated(id, component_id, expected, actor)
            .await
    }

    pub async fn mark_component_not_eaten_backdated(
        &self,
        id: MealPlanEntryId,
        component_id: MealPlanComponentId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        let subject = self.resolve_subject(&entry, actor.subject_member_id)?;
        let component = find_component(&entry, component_id)?;
        require_revision(
            MEAL_PLAN_COMPONENT,
            component_id,
            expected,
            component.revision,
        )?;
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
        let update = component_update(
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
        commit_outcome(MEAL_PLAN_COMPONENT, component_id, expected, outcome)?;
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
        require_revision(
            MEAL_PLAN_COMPONENT,
            component_id,
            expected,
            component.revision,
        )?;
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
        let still_eaten = component_still_eaten(&entry, component_id);
        let snapshot_op = if still_eaten {
            SnapshotOp::Keep
        } else {
            SnapshotOp::Clear
        };
        let write = StockWrite {
            deductions: Vec::new(),
            releases: {
                self.component_release(
                    &catalogue,
                    &entry,
                    component_id,
                    component_item,
                    actor.actor_id,
                    subject,
                    still_eaten,
                )
                .into_iter()
                .collect()
            },
        };
        let update = component_update(
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
        commit_outcome(MEAL_PLAN_COMPONENT, component_id, expected, outcome)?;
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
        self.mark_not_eaten_backdated(id, expected, actor).await
    }

    pub async fn mark_not_eaten_backdated(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        require_revision(MEAL_PLAN_ENTRY, id, expected, entry.revision)?;
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
        let (outcome, stock_outcomes) = self
            .plans
            .resolve(&entry, expected, &[], &StockWrite::default())
            .await?;
        commit_outcome(MEAL_PLAN_ENTRY, id, expected, outcome)?;
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
        self.mark_eaten_backdated(id, expected, input).await
    }

    pub async fn mark_eaten_backdated(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ConfirmMealPlanEntry,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        require_revision(MEAL_PLAN_ENTRY, id, expected, entry.revision)?;
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
            deductions.extend(
                self.component_deduction(
                    &catalogue,
                    &entry,
                    *component_id,
                    component.item,
                    &component.amount,
                    &amount,
                    subject.as_uuid(),
                    component_still_eaten(&entry, *component_id),
                    input.actor_id,
                    Some(subject),
                )
                .await?,
            );
            let record = ConsumptionRecord::create(
                NewConsumptionRecord {
                    id: None,
                    member_id: subject,
                    item: component.item,
                    recorded_by: Some(input.actor_id),
                    meal_plan_entry_id: Some(entry.id),
                    meal_plan_component_id: Some(component.id),
                    slot: entry.slot,
                    amount,
                    consumed_on: input.consumed_on,
                    consumed_at: input.consumed_at,
                },
                scaled.nutrition.facts,
                scaled.nutrition.quality,
                now,
            );
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
        let write = StockWrite {
            deductions,
            releases: Vec::new(),
        };
        let (outcome, stock_outcomes) = self
            .plans
            .resolve(&entry, expected, &records, &write)
            .await?;
        commit_outcome(MEAL_PLAN_ENTRY, id, expected, outcome)?;
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
        self.review_outcomes_backdated(id, expected, input).await
    }

    pub async fn review_outcomes_backdated(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ReviewMealOutcomes,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        require_revision(MEAL_PLAN_ENTRY, id, expected, entry.revision)?;
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
                    deductions.extend(
                        self.component_deduction(
                            &catalogue,
                            &entry,
                            component_id,
                            component.item,
                            &component.amount,
                            &amount,
                            reviewed.member_id.as_uuid(),
                            component_still_eaten(&entry, component_id),
                            input.actor_id,
                            Some(reviewed.member_id),
                        )
                        .await?,
                    );
                    let record = ConsumptionRecord::create(
                        NewConsumptionRecord {
                            id: None,
                            member_id: reviewed.member_id,
                            item: component.item,
                            recorded_by: Some(input.actor_id),
                            meal_plan_entry_id: Some(entry.id),
                            meal_plan_component_id: Some(component.id),
                            slot: entry.slot,
                            amount,
                            consumed_on: input.consumed_on,
                            consumed_at: input.consumed_at,
                        },
                        scaled.nutrition.facts,
                        scaled.nutrition.quality,
                        now,
                    );
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
                let record = ConsumptionRecord::create(
                    NewConsumptionRecord {
                        id: None,
                        member_id: reviewed.member_id,
                        item: replacement.item,
                        recorded_by: Some(input.actor_id),
                        meal_plan_entry_id: Some(entry.id),
                        meal_plan_component_id: None,
                        slot: entry.slot,
                        amount: replacement.amount,
                        consumed_on: input.consumed_on,
                        consumed_at: input.consumed_at,
                    },
                    scaled.nutrition.facts,
                    scaled.nutrition.quality,
                    now,
                );
                deductions.extend(self.record_deduction_for(&catalogue, &record));
                records.push(record);
            }
        }

        let guest_results = build_guest_results(&entry, &input, now)?;
        let mut guest_deductions = HashSet::new();
        for group in &guest_results {
            for allocation in &group.allocations {
                if allocation.status == ParticipantStatus::Eaten
                    && guest_deductions.insert(allocation.id)
                {
                    let component = find_component(&entry, allocation.component_id)?.clone();
                    deductions.extend(
                        self.component_deduction(
                            &catalogue,
                            &entry,
                            component.id,
                            component.item,
                            &component.amount,
                            &allocation.allocated,
                            allocation.id.as_uuid(),
                            component_still_eaten(&entry, allocation.component_id),
                            input.actor_id,
                            None,
                        )
                        .await?,
                    );
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
        let write = StockWrite {
            deductions,
            releases: Vec::new(),
        };
        let (outcome, stock_outcomes) = self
            .plans
            .resolve(&entry, expected, &records, &write)
            .await?;
        commit_outcome(MEAL_PLAN_ENTRY, id, expected, outcome)?;
        self.stock_affected(id, stock_outcomes).await
    }

    pub async fn reopen(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor: OutcomeActor,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let mut entry = self.get_entry(id).await?;
        require_revision(MEAL_PLAN_ENTRY, id, expected, entry.revision)?;
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
            let item = entry
                .components
                .iter()
                .find(|c| c.id == component_id)
                .map(|c| c.item);
            if let Some(item) = item {
                releases.extend(self.component_release(
                    &catalogue,
                    &entry,
                    component_id,
                    item,
                    actor.actor_id,
                    subject,
                    component_still_eaten(&entry, component_id),
                ));
            }
        }
        entry.updated_by = actor.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        let write = StockWrite {
            deductions: Vec::new(),
            releases,
        };
        let (outcome, stock_outcomes) = self
            .plans
            .reopen(&entry, expected, &record_ids, &write)
            .await?;
        commit_outcome(MEAL_PLAN_ENTRY, id, expected, outcome)?;
        self.stock_affected(id, stock_outcomes).await
    }

    async fn stock_affected(
        &self,
        id: MealPlanEntryId,
        outcomes: Vec<StockOutcome>,
    ) -> Result<StockAffected<MealPlanEntryView>> {
        let view = self.get(id).await?;
        let named = name_outcomes(
            &*self.products,
            &*self.ingredients,
            &*self.batches,
            outcomes,
        )
        .await?;
        Ok(StockAffected::new(view, named))
    }

    #[allow(clippy::too_many_arguments)]
    async fn component_deduction(
        &self,
        catalogue: &ItemCatalogue,
        entry: &MealPlanEntry,
        component_id: MealPlanComponentId,
        item: MealItemRef,
        prepared_amount: &ConsumedAmount,
        eaten_amount: &ConsumedAmount,
        eater_id: uuid::Uuid,
        already_drawn: bool,
        actor: UserId,
        subject: Option<HouseholdMemberId>,
    ) -> Result<Vec<StockDeduction>> {
        match item {
            MealItemRef::Product { product_id } => {
                if already_drawn {
                    return Ok(Vec::new());
                }
                let Some(product) = catalogue.products.get(&product_id) else {
                    return Ok(Vec::new());
                };
                Ok(product_deduction(
                    StockEffectSource::MealPlanComponent,
                    component_id.as_uuid(),
                    product,
                    prepared_amount,
                    stock_source_label(entry, &product.name),
                    Some(actor),
                    subject,
                )
                .into_iter()
                .collect())
            }
            MealItemRef::Recipe { .. } => {
                let batch = self
                    .cooked_batch(component_id, &catalogue.name_of(item))
                    .await?;
                let ConsumedAmount::Servings(servings) = *eaten_amount else {
                    return Ok(Vec::new());
                };
                Ok(vec![portion_deduction(
                    component_id.as_uuid(),
                    eater_id,
                    batch.id,
                    Quantity::new(servings, Unit::Serving),
                    stock_source_label(entry, &batch.item_name),
                    Some(actor),
                    subject,
                )])
            }
            MealItemRef::Dish { recipe_id } => {
                let ConsumedAmount::Servings(servings) = *eaten_amount else {
                    return Ok(Vec::new());
                };
                let name = catalogue.name_of(item);
                Ok(vec![cooked_food_deduction(
                    component_id.as_uuid(),
                    eater_id,
                    recipe_id,
                    Quantity::new(servings, Unit::Serving),
                    stock_source_label(entry, &name),
                    Some(actor),
                    subject,
                )])
            }
        }
    }

    async fn cooked_batch(
        &self,
        component_id: MealPlanComponentId,
        name: &str,
    ) -> Result<PreparedBatch> {
        self.preparation
            .for_component(component_id)
            .await?
            .ok_or_else(|| {
                CoreError::conflict(format!("Record that you cooked {name} before eating it."))
            })
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
            MealItemRef::Dish { recipe_id } => {
                let ConsumedAmount::Servings(servings) = record.amount else {
                    return Vec::new();
                };
                vec![cooked_food_deduction(
                    record.id.as_uuid(),
                    record.member_id.as_uuid(),
                    recipe_id,
                    Quantity::new(servings, Unit::Serving),
                    catalogue.name_of(record.item),
                    record.recorded_by,
                    Some(record.member_id),
                )]
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
                MealItemRef::Dish { recipe_id } => {
                    if self.batches.held_for_recipe(recipe_id).await?.is_empty() {
                        return Err(CoreError::not_found(DISH, recipe_id));
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn component_release(
        &self,
        catalogue: &ItemCatalogue,
        entry: &MealPlanEntry,
        component_id: MealPlanComponentId,
        item: MealItemRef,
        actor: UserId,
        subject: HouseholdMemberId,
        still_eaten: bool,
    ) -> Option<StockRelease> {
        let (name, eater_id) = match item {
            MealItemRef::Product { product_id } => {
                if still_eaten {
                    return None;
                }
                let name = catalogue
                    .products
                    .get(&product_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "food".to_owned());
                (name, None)
            }
            MealItemRef::Recipe { .. } | MealItemRef::Dish { .. } => {
                ("food".to_owned(), Some(subject.as_uuid()))
            }
        };
        Some(component_release(
            component_id.as_uuid(),
            eater_id,
            stock_source_label(entry, &name),
            Some(actor),
            Some(subject),
        ))
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
}

fn component_update<'a>(
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

fn stock_source_label(entry: &MealPlanEntry, item_name: &str) -> String {
    let slot = match entry.slot {
        MealSlot::Breakfast => "Breakfast",
        MealSlot::Lunch => "Lunch",
        MealSlot::Dinner => "Dinner",
        MealSlot::Snacks => "Snacks",
    };
    format!("{slot} {} \u{2014} {item_name}", entry.planned_on)
}
