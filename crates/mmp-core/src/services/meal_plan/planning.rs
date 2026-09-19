use std::collections::HashSet;

use time::{Date, Duration, OffsetDateTime};

use crate::domain::{
    HouseholdMemberId, MEAL_OCCASION, MEAL_PLAN_ENTRY, MealAbsence, MealAttendance, MealGroupPatch,
    MealGuestAllocationId, MealGuestGroupId, MealOccasion, MealOccasionId, MealOccasionPatch,
    MealPlanEntry, MealPlanEntryId, MealSlot, NewMealGroup, NewMealGuestGroup, NewMealOccasion,
    NewMealParticipant, NewMealPlanComponent, ParticipantStatus, Revision, UserId,
    apply_equal_shares, has_explicit_allocations, make_components, merge_components,
    merge_guest_group, merge_participant, require_editable, require_planned, sync_allocations,
    validate_components, validate_group_shape, validate_guest_groups, validate_participants,
};
use crate::error::{CoreError, Result, ValidationErrors};

use super::view::{MealOccasionView, PlannerWeek};
use super::{MealPlanService, ensure_not_past};
use crate::services::revision::{commit_outcome, require_revision};

pub enum GuestMealTarget {
    Existing(MealPlanEntryId),
    New(NewMealGroup),
}

pub enum GuestChange {
    Update {
        name: Option<Option<String>>,
        note: Option<Option<String>>,
        target: Option<GuestMealTarget>,
    },
    Remove,
}

impl MealPlanService {
    pub async fn create_occasion(&self, input: NewMealOccasion) -> Result<MealOccasionView> {
        ensure_not_past(&self.clock, &*self.settings, input.planned_on).await?;
        self.create_occasion_backdated(input).await
    }

    pub async fn create_occasion_backdated(
        &self,
        input: NewMealOccasion,
    ) -> Result<MealOccasionView> {
        let now = self.clock.now();
        if let Some(existing) = self
            .plans
            .find_occasion(input.planned_on, input.slot)
            .await?
        {
            let mut occasion = self.load_occasion(existing.id).await?;
            let group = self
                .build_group(&occasion, input.group, input.actor_id, now)
                .await?;
            let expected = occasion.revision;
            attach_group(&mut occasion, group)?;
            self.commit_occasion(&mut occasion, expected, input.actor_id, now)
                .await?;
            return self.get_occasion(occasion.id).await;
        }

        let mut occasion = MealOccasion {
            id: input.id.unwrap_or_default(),
            planned_on: input.planned_on,
            slot: input.slot,
            planned_time: input.planned_time,
            note: normalise_note(input.note),
            groups: Vec::new(),
            absences: Vec::new(),
            created_by: input.actor_id,
            updated_by: input.actor_id,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        let group = self
            .build_group(&occasion, input.group, input.actor_id, now)
            .await?;
        attach_group(&mut occasion, group)?;
        let active = self.active_member_ids().await?;
        self.materialise_occasion(&mut occasion, &active);
        self.plans.insert_occasion(&occasion).await?;
        self.get_occasion(occasion.id).await
    }

    pub async fn update_occasion(
        &self,
        id: MealOccasionId,
        expected: Revision,
        patch: MealOccasionPatch,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let mut occasion = self.load_occasion(id).await?;
        require_revision(MEAL_OCCASION, id, expected, occasion.revision)?;
        if let Some(planned_time) = patch.planned_time {
            occasion.planned_time = planned_time;
        }
        if let Some(note) = patch.note {
            occasion.note = normalise_note(note);
        }
        let now = self.clock.now();
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(id).await
    }

    pub async fn delete_occasion(&self, id: MealOccasionId, expected: Revision) -> Result<()> {
        let occasion = self.load_occasion(id).await?;
        require_revision(MEAL_OCCASION, id, expected, occasion.revision)?;
        for group in &occasion.groups {
            require_planned(group)?;
        }
        commit_outcome(
            MEAL_OCCASION,
            id,
            expected,
            self.plans.delete_occasion(id, expected).await?,
        )
    }

    pub async fn move_occasion(
        &self,
        id: MealOccasionId,
        expected: Revision,
        planned_on: Date,
        slot: MealSlot,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        ensure_not_past(&self.clock, &*self.settings, planned_on).await?;
        let mut occasion = self.load_occasion(id).await?;
        require_revision(MEAL_OCCASION, id, expected, occasion.revision)?;
        if occasion.planned_on == planned_on && occasion.slot == slot {
            return self.get_occasion(id).await;
        }
        for group in &occasion.groups {
            require_planned(group)?;
        }
        self.ensure_cell_free(planned_on, slot).await?;
        occasion.planned_on = planned_on;
        occasion.slot = slot;
        let now = self.clock.now();
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(id).await
    }

    pub async fn copy_occasion(
        &self,
        id: MealOccasionId,
        planned_on: Date,
        slot: MealSlot,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        ensure_not_past(&self.clock, &*self.settings, planned_on).await?;
        let source = self.load_occasion(id).await?;
        self.ensure_cell_free(planned_on, slot).await?;
        let now = self.clock.now();
        let mut copy = copy_of(&source, planned_on, slot, actor_id, now);
        let active = self.active_member_ids().await?;
        self.materialise_occasion(&mut copy, &active);
        self.plans.insert_occasion(&copy).await?;
        self.get_occasion(copy.id).await
    }

    pub async fn copy_week(
        &self,
        target_week_start: Date,
        source_week_start: Date,
        actor_id: UserId,
    ) -> Result<PlannerWeek> {
        ensure_not_past(&self.clock, &*self.settings, target_week_start).await?;
        if target_week_start == source_week_start {
            return self.planner_week(target_week_start).await;
        }
        let sources = self
            .plans
            .list_occasions(source_week_start, source_week_start + Duration::days(6))
            .await?;
        let existing = self
            .plans
            .list_occasions(target_week_start, target_week_start + Duration::days(6))
            .await?;
        let taken: HashSet<(Date, MealSlot)> = existing
            .iter()
            .map(|occasion| (occasion.planned_on, occasion.slot))
            .collect();
        let now = self.clock.now();
        let active = self.active_member_ids().await?;
        for source in sources {
            let planned_on = target_week_start + (source.planned_on - source_week_start);
            if taken.contains(&(planned_on, source.slot)) {
                continue;
            }
            let mut copy = copy_of(&source, planned_on, source.slot, actor_id, now);
            self.materialise_occasion(&mut copy, &active);
            self.plans.insert_occasion(&copy).await?;
        }
        self.planner_week(target_week_start).await
    }

    pub async fn add_group(
        &self,
        occasion_id: MealOccasionId,
        input: NewMealGroup,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let mut occasion = self.load_occasion(occasion_id).await?;
        let now = self.clock.now();
        let group = self.build_group(&occasion, input, actor_id, now).await?;
        let expected = occasion.revision;
        attach_group(&mut occasion, group)?;
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(occasion_id).await
    }

    pub async fn add_guest(
        &self,
        occasion_id: MealOccasionId,
        expected: Revision,
        name: Option<String>,
        note: Option<String>,
        target: GuestMealTarget,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let mut occasion = self.load_occasion(occasion_id).await?;
        require_revision(MEAL_OCCASION, occasion_id, expected, occasion.revision)?;
        let now = self.clock.now();
        let name = normalise_note(name);
        let note = normalise_note(note);
        match target {
            GuestMealTarget::Existing(group_id) => {
                let group = occasion
                    .groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, group_id))?;
                require_planned(group)?;
                group.guest_groups.push(merge_guest_group(
                    &[],
                    &NewMealGuestGroup {
                        id: None,
                        count: 1,
                        name,
                        note,
                        allocations: Vec::new(),
                    },
                    now,
                ));
                sync_allocations(group, now);
                apply_equal_shares(group);
                group.revision = group.revision.next();
                group.updated_by = actor_id;
                group.updated_at = now;
            }
            GuestMealTarget::New(mut input) => {
                input.everyone = false;
                input.participants.clear();
                input.guest_groups = vec![NewMealGuestGroup {
                    id: None,
                    count: 1,
                    name,
                    note,
                    allocations: Vec::new(),
                }];
                let group = self.build_group(&occasion, input, actor_id, now).await?;
                attach_group(&mut occasion, group)?;
            }
        }
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(occasion_id).await
    }

    pub async fn change_guest(
        &self,
        occasion_id: MealOccasionId,
        expected: Revision,
        guest_id: MealGuestGroupId,
        change: GuestChange,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let (name, note, target, remove) = match change {
            GuestChange::Update { name, note, target } => (name, note, target, false),
            GuestChange::Remove => (None, None, None, true),
        };
        let mut occasion = self.load_occasion(occasion_id).await?;
        require_revision(MEAL_OCCASION, occasion_id, expected, occasion.revision)?;
        let now = self.clock.now();
        let source_index = occasion
            .groups
            .iter()
            .position(|group| group.guest_groups.iter().any(|guest| guest.id == guest_id))
            .ok_or_else(|| CoreError::not_found("guest", guest_id))?;
        let source = &mut occasion.groups[source_index];
        require_planned(source)?;
        let guest_index = source
            .guest_groups
            .iter()
            .position(|guest| guest.id == guest_id)
            .ok_or_else(|| CoreError::not_found("guest", guest_id))?;
        let guest = &source.guest_groups[guest_index];
        if guest.count != 1 {
            return Err(CoreError::conflict(
                "Split this guest count before editing one guest.",
            ));
        }
        if guest
            .allocations
            .iter()
            .any(|allocation| allocation.status.is_resolved())
        {
            return Err(CoreError::conflict(
                "A guest result has already been recorded.",
            ));
        }
        let mut guest = source.guest_groups.remove(guest_index);
        if let Some(name) = name {
            guest.name = normalise_note(name);
        }
        if let Some(note) = note {
            guest.note = normalise_note(note);
        }
        guest.revision = guest.revision.next();
        guest.updated_at = now;
        apply_equal_shares(source);
        source.revision = source.revision.next();
        source.updated_by = actor_id;
        source.updated_at = now;
        if !remove {
            match target {
                Some(GuestMealTarget::Existing(group_id)) if group_id != source.id => {
                    let destination = occasion
                        .groups
                        .iter_mut()
                        .find(|group| group.id == group_id)
                        .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, group_id))?;
                    require_planned(destination)?;
                    guest.allocations.clear();
                    destination.guest_groups.push(guest);
                    sync_allocations(destination, now);
                    apply_equal_shares(destination);
                    destination.revision = destination.revision.next();
                    destination.updated_by = actor_id;
                    destination.updated_at = now;
                }
                Some(GuestMealTarget::New(mut input)) => {
                    input.everyone = false;
                    input.participants.clear();
                    input.guest_groups.clear();
                    let mut destination = self.build_group(&occasion, input, actor_id, now).await?;
                    guest.allocations.clear();
                    destination.guest_groups.push(guest);
                    sync_allocations(&mut destination, now);
                    apply_equal_shares(&mut destination);
                    attach_group(&mut occasion, destination)?;
                }
                _ => {
                    let source = &mut occasion.groups[source_index];
                    source.guest_groups.push(guest);
                    apply_equal_shares(source);
                }
            }
        }
        drop_orphaned_groups(&mut occasion);
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(occasion_id).await
    }

    pub async fn split_guests(
        &self,
        occasion_id: MealOccasionId,
        expected: Revision,
        guest_id: MealGuestGroupId,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let mut occasion = self.load_occasion(occasion_id).await?;
        require_revision(MEAL_OCCASION, occasion_id, expected, occasion.revision)?;
        let group = occasion
            .groups
            .iter_mut()
            .find(|group| group.guest_groups.iter().any(|guest| guest.id == guest_id))
            .ok_or_else(|| CoreError::not_found("guest", guest_id))?;
        let guest_index = group
            .guest_groups
            .iter()
            .position(|guest| guest.id == guest_id)
            .ok_or_else(|| CoreError::not_found("guest", guest_id))?;
        let source = group.guest_groups[guest_index].clone();
        let now = self.clock.now();
        if source
            .allocations
            .iter()
            .any(|allocation| allocation.status.is_resolved())
        {
            return Err(CoreError::conflict("Recorded guests cannot be split."));
        }
        if source.count <= 1 {
            return self.get_occasion(occasion_id).await;
        }
        group.guest_groups[guest_index].count = 1;
        group.guest_groups[guest_index].revision = group.guest_groups[guest_index].revision.next();
        group.guest_groups[guest_index].updated_at = now;
        for _ in 1..source.count {
            let mut guest = source.clone();
            guest.id = MealGuestGroupId::new();
            guest.count = 1;
            guest.name = None;
            guest.note = None;
            guest.revision = Revision::INITIAL;
            guest.updated_at = now;
            for allocation in &mut guest.allocations {
                allocation.id = MealGuestAllocationId::new();
            }
            group.guest_groups.push(guest);
        }
        group.revision = group.revision.next();
        group.updated_by = actor_id;
        group.updated_at = now;
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(occasion_id).await
    }

    pub async fn update_group(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        patch: MealGroupPatch,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let stored = self
            .plans
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))?;
        let mut occasion = self.load_occasion(stored.occasion_id).await?;
        let occasion_revision = occasion.revision;
        let index = occasion
            .groups
            .iter()
            .position(|group| group.id == id)
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))?;
        let now = self.clock.now();
        let mut group = occasion.groups[index].clone();
        require_revision(MEAL_PLAN_ENTRY, id, expected, group.revision)?;
        require_editable(&group)?;

        let mut reshare = false;
        if let Some(label) = patch.label {
            group.label = normalise_note(label);
        }
        if let Some(ad_hoc) = patch.ad_hoc {
            group.ad_hoc = ad_hoc;
        }
        if let Some(components) = patch.components {
            validate_group_shape(group.label.as_deref(), group.ad_hoc, &components)?;
            validate_components(&components)?;
            let existing_items = group
                .components
                .iter()
                .map(|component| component.item)
                .collect();
            self.validate_component_items(&components, &existing_items, actor_id)
                .await?;
            group.components = merge_components(&group.components, components)?;
            sync_allocations(&mut group, now);
            reshare = true;
        } else {
            let shape: Vec<NewMealPlanComponent> = group
                .components
                .iter()
                .map(|component| NewMealPlanComponent {
                    id: Some(component.id),
                    item: component.item,
                    amount: component.amount,
                })
                .collect();
            validate_group_shape(group.label.as_deref(), group.ad_hoc, &shape)?;
        }
        if let Some(everyone) = patch.everyone {
            if everyone
                && occasion
                    .everyone_group()
                    .is_some_and(|other| other.id != group.id)
            {
                return Err(CoreError::conflict(
                    "Another meal here is already for everyone. Untick people from it instead.",
                ));
            }
            group.everyone = everyone;
        }
        if let Some(participants) = patch.participants {
            validate_participants(&participants, &group.components)?;
            let requested_member_ids: Vec<HouseholdMemberId> =
                participants.iter().map(|p| p.member_id).collect();
            self.ensure_members_active(requested_member_ids).await?;
            if let Some(removed) = group.participants.iter().find(|existing| {
                !participants
                    .iter()
                    .any(|kept| kept.member_id == existing.member_id)
                    && existing.allocations.iter().any(|a| a.status.is_resolved())
            }) {
                let _ = removed;
                return Err(CoreError::conflict(
                    "Someone who has already eaten cannot be removed. Reopen their portion first.",
                ));
            }
            let explicit = has_explicit_allocations(&participants);
            group.participants = participants
                .iter()
                .map(|participant| {
                    merge_participant(
                        group.participant_for(participant.member_id),
                        participant,
                        &group.components,
                        now,
                    )
                })
                .collect();
            reshare |= !explicit;
        }
        if let Some(guest_groups) = patch.guest_groups {
            validate_guest_groups(&guest_groups, &group.components)?;
            group.guest_groups = guest_groups
                .iter()
                .map(|guests| merge_guest_group(&group.guest_groups, guests, now))
                .collect();
            sync_allocations(&mut group, now);
            reshare = true;
        }
        if let Some(cooking_servings) = patch.cooking_servings {
            validate_cooking_servings(cooking_servings)?;
            group.cooking_servings = cooking_servings;
        }
        if reshare {
            apply_equal_shares(&mut group);
        }
        group.updated_by = actor_id;
        group.updated_at = now;
        group.revision = group.revision.next();

        let members: Vec<HouseholdMemberId> = group
            .participants
            .iter()
            .map(|participant| participant.member_id)
            .collect();
        occasion.groups[index] = group;
        claim_members(&mut occasion, id, &members)?;
        drop_orphaned_groups(&mut occasion);
        self.commit_occasion(&mut occasion, occasion_revision, actor_id, now)
            .await?;
        self.get_occasion(occasion.id).await
    }

    pub async fn delete_group(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor_id: UserId,
    ) -> Result<Option<MealOccasionView>> {
        let stored = self
            .plans
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))?;
        let mut occasion = self.load_occasion(stored.occasion_id).await?;
        let group = occasion
            .group(id)
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))?;
        require_revision(MEAL_PLAN_ENTRY, id, expected, group.revision)?;
        require_planned(group)?;
        occasion.groups.retain(|group| group.id != id);
        if occasion.groups.is_empty() {
            commit_outcome(
                MEAL_OCCASION,
                occasion.id,
                occasion.revision,
                self.plans
                    .delete_occasion(occasion.id, occasion.revision)
                    .await?,
            )?;
            return Ok(None);
        }
        let expected_occasion = occasion.revision;
        let now = self.clock.now();
        self.commit_occasion(&mut occasion, expected_occasion, actor_id, now)
            .await?;
        Ok(Some(self.get_occasion(occasion.id).await?))
    }

    pub async fn set_attendance(
        &self,
        occasion_id: MealOccasionId,
        member_id: HouseholdMemberId,
        attendance: MealAttendance,
        actor_id: UserId,
    ) -> Result<MealOccasionView> {
        let mut occasion = self.load_occasion(occasion_id).await?;
        self.ensure_members_active([member_id]).await?;
        let expected = occasion.revision;
        let now = self.clock.now();
        match attendance {
            MealAttendance::Eating { group_id, note } => {
                release_member(&mut occasion, Some(group_id), member_id)?;
                let index = occasion
                    .groups
                    .iter()
                    .position(|group| group.id == group_id)
                    .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, group_id))?;
                let group = &mut occasion.groups[index];
                let requested = NewMealParticipant {
                    id: None,
                    member_id,
                    note: note.clone(),
                    allocations: Vec::new(),
                };
                let merged = merge_participant(
                    group.participant_for(member_id),
                    &requested,
                    &group.components,
                    now,
                );
                match group
                    .participants
                    .iter_mut()
                    .find(|participant| participant.member_id == member_id)
                {
                    Some(existing) => {
                        existing.note = note;
                        existing.updated_at = now;
                    }
                    None => group.participants.push(merged),
                }
                apply_equal_shares(group);
                group.updated_by = actor_id;
                group.updated_at = now;
                group.revision = group.revision.next();
            }
            MealAttendance::Elsewhere => {
                release_member(&mut occasion, None, member_id)?;
                occasion.absences.push(MealAbsence {
                    member_id,
                    created_by: actor_id,
                    created_at: now,
                });
            }
            MealAttendance::Unaccounted => {
                release_member(&mut occasion, None, member_id)?;
            }
        }
        self.commit_occasion(&mut occasion, expected, actor_id, now)
            .await?;
        self.get_occasion(occasion_id).await
    }

    async fn ensure_cell_free(&self, planned_on: Date, slot: MealSlot) -> Result<()> {
        if self.plans.find_occasion(planned_on, slot).await?.is_some() {
            return Err(CoreError::conflict(
                "There is already a meal planned there. Move or delete it first.",
            ));
        }
        Ok(())
    }

    async fn ensure_members_active(
        &self,
        member_ids: impl IntoIterator<Item = HouseholdMemberId>,
    ) -> Result<()> {
        for member_id in member_ids {
            let member = self
                .members
                .get(member_id)
                .await?
                .ok_or_else(|| CoreError::not_found("household member", member_id))?;
            if member.is_archived() {
                let mut errors = ValidationErrors::new();
                errors.push("participants", "Archived members cannot join a meal");
                return Err(errors.into());
            }
        }
        Ok(())
    }

    async fn build_group(
        &self,
        occasion: &MealOccasion,
        input: NewMealGroup,
        actor_id: UserId,
        now: OffsetDateTime,
    ) -> Result<MealPlanEntry> {
        validate_group_shape(input.label.as_deref(), input.ad_hoc, &input.components)?;
        validate_components(&input.components)?;
        self.validate_component_items(&input.components, &HashSet::new(), actor_id)
            .await?;
        validate_cooking_servings(input.cooking_servings)?;
        if input.everyone
            && occasion
                .everyone_group()
                .is_some_and(|other| Some(other.id) != input.id)
        {
            return Err(CoreError::conflict(
                "This meal already has a group for everyone. Untick people from it instead.",
            ));
        }
        let components = make_components(input.components);
        validate_participants(&input.participants, &components)?;
        let requested_member_ids: Vec<HouseholdMemberId> =
            input.participants.iter().map(|p| p.member_id).collect();
        self.ensure_members_active(requested_member_ids).await?;
        validate_guest_groups(&input.guest_groups, &components)?;
        let explicit = has_explicit_allocations(&input.participants);
        let participants = input
            .participants
            .iter()
            .map(|participant| merge_participant(None, participant, &components, now))
            .collect();
        let guest_groups = input
            .guest_groups
            .iter()
            .map(|group| merge_guest_group(&[], group, now))
            .collect();
        let mut group = MealPlanEntry {
            id: input.id.unwrap_or_default(),
            occasion_id: occasion.id,
            planned_on: occasion.planned_on,
            planned_time: occasion.planned_time,
            slot: occasion.slot,
            label: normalise_note(input.label),
            ad_hoc: input.ad_hoc,
            components,
            everyone: input.everyone,
            participants,
            guest_groups,
            cooking_servings: input.cooking_servings,
            created_by: actor_id,
            updated_by: actor_id,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        sync_allocations(&mut group, now);
        if !explicit {
            apply_equal_shares(&mut group);
        }
        Ok(group)
    }

    async fn commit_occasion(
        &self,
        occasion: &mut MealOccasion,
        expected: Revision,
        actor_id: UserId,
        now: OffsetDateTime,
    ) -> Result<()> {
        occasion.updated_by = actor_id;
        occasion.updated_at = now;
        occasion.revision = occasion.revision.next();
        let active = self.active_member_ids().await?;
        self.materialise_occasion(occasion, &active);
        for group in &mut occasion.groups {
            group.planned_on = occasion.planned_on;
            group.planned_time = occasion.planned_time;
            group.slot = occasion.slot;
        }
        commit_outcome(
            MEAL_OCCASION,
            occasion.id,
            expected,
            self.plans.update_occasion(occasion, expected).await?,
        )
    }
}

fn normalise_note(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn validate_cooking_servings(value: Option<i32>) -> Result<()> {
    if value.is_some_and(|value| value <= 0) {
        let mut errors = ValidationErrors::new();
        errors.push("cooking_servings", "Cook at least one serving");
        return errors.into_result();
    }
    Ok(())
}

fn attach_group(occasion: &mut MealOccasion, group: MealPlanEntry) -> Result<()> {
    if group.everyone && occasion.everyone_group().is_some() {
        return Err(CoreError::conflict(
            "This meal already has a group for everyone. Untick people from it instead.",
        ));
    }
    let members: Vec<HouseholdMemberId> = group
        .participants
        .iter()
        .map(|participant| participant.member_id)
        .collect();
    let group_id = group.id;
    occasion.groups.push(group);
    claim_members(occasion, group_id, &members)
}

fn claim_members(
    occasion: &mut MealOccasion,
    group_id: MealPlanEntryId,
    members: &[HouseholdMemberId],
) -> Result<()> {
    for member_id in members {
        release_member(occasion, Some(group_id), *member_id)?;
    }
    Ok(())
}

fn release_member(
    occasion: &mut MealOccasion,
    keep: Option<MealPlanEntryId>,
    member_id: HouseholdMemberId,
) -> Result<()> {
    for group in &mut occasion.groups {
        if Some(group.id) == keep {
            continue;
        }
        let Some(participant) = group.participant_for(member_id) else {
            continue;
        };
        if participant
            .allocations
            .iter()
            .any(|allocation| allocation.status != ParticipantStatus::Planned)
        {
            return Err(CoreError::conflict(
                "That person has already recorded this meal. Reopen it before moving them.",
            ));
        }
        group
            .participants
            .retain(|participant| participant.member_id != member_id);
        apply_equal_shares(group);
    }
    occasion
        .absences
        .retain(|absence| absence.member_id != member_id);
    drop_orphaned_groups(occasion);
    Ok(())
}

fn is_orphaned_group(group: &MealPlanEntry) -> bool {
    !group.everyone && group.participants.is_empty() && group.guest_count() == 0
}

fn drop_orphaned_groups(occasion: &mut MealOccasion) {
    let mut removable: Vec<MealPlanEntryId> = occasion
        .groups
        .iter()
        .filter(|group| is_orphaned_group(group))
        .map(|group| group.id)
        .collect();
    removable.truncate(occasion.groups.len().saturating_sub(1));
    if removable.is_empty() {
        return;
    }
    occasion
        .groups
        .retain(|group| !removable.contains(&group.id));
}

fn copy_of(
    source: &MealOccasion,
    planned_on: Date,
    slot: MealSlot,
    actor_id: UserId,
    now: OffsetDateTime,
) -> MealOccasion {
    let id = MealOccasionId::new();
    let groups = source
        .groups
        .iter()
        .map(|group| {
            let components = make_components(
                group
                    .components
                    .iter()
                    .map(|component| NewMealPlanComponent {
                        id: None,
                        item: component.item,
                        amount: component.amount,
                    })
                    .collect(),
            );
            let participants = group
                .participants
                .iter()
                .map(|participant| {
                    merge_participant(
                        None,
                        &NewMealParticipant {
                            id: None,
                            member_id: participant.member_id,
                            note: participant.note.clone(),
                            allocations: Vec::new(),
                        },
                        &components,
                        now,
                    )
                })
                .collect();
            let mut copy = MealPlanEntry {
                id: MealPlanEntryId::new(),
                occasion_id: id,
                planned_on,
                planned_time: source.planned_time,
                slot,
                label: group.label.clone(),
                ad_hoc: group.ad_hoc,
                components,
                everyone: group.everyone,
                participants,
                guest_groups: Vec::new(),
                cooking_servings: None,
                created_by: actor_id,
                updated_by: actor_id,
                revision: Revision::INITIAL,
                created_at: now,
                updated_at: now,
            };
            apply_equal_shares(&mut copy);
            copy
        })
        .collect();
    MealOccasion {
        id,
        planned_on,
        slot,
        planned_time: source.planned_time,
        note: source.note.clone(),
        groups,
        absences: Vec::new(),
        created_by: actor_id,
        updated_by: actor_id,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}
