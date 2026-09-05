use std::collections::HashSet;

use time::{Date, Time};

use crate::domain::{
    HouseholdMemberId, MealOptOut, MealPlanEntry, MealPlanEntryId, MealPlanEntryPatch,
    MealPlanScope, MealSlot, NewMealPlanEntry, Portioning, Revision, SetMealParticipants,
    SlotAttendance, UserId, apply_equal_portioning, build_participant, has_explicit_allocations,
    make_components, merge_components, merge_guest_group, merge_participant,
    require_household_attendance, require_planned, sync_allocations, validate_components,
    validate_guest_groups, validate_participants,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{MealPlanQuery, MemberQuery, PageRequest};

use super::view::MealPlanEntryView;
use super::{MealPlanService, commit_outcome, ensure_not_past, require_editable, require_revision};

impl MealPlanService {
    pub async fn create(&self, input: NewMealPlanEntry) -> Result<MealPlanEntryView> {
        ensure_not_past(&*self.clock, input.planned_on)?;
        self.create_backdated(input).await
    }

    pub async fn create_backdated(&self, input: NewMealPlanEntry) -> Result<MealPlanEntryView> {
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
}
