use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::{
    Assumption, MealGuestAllocation, MealGuestGroup, MealParticipant, MealParticipantAllocation,
    MealPlanComponent, MealPlanEntry, MealPlanScope, MealPlanStatus, NewMealGuestGroup,
    NewMealParticipant, ParticipantStatus, Portioning, equal_split,
};
use crate::domain::{
    ConsumedAmount, ConsumptionRecordId, HouseholdMemberId, MealParticipantAllocationId,
    MealParticipantId, MealPlanComponentId, Revision, UserId,
};
use crate::error::{CoreError, Result, ValidationErrors};

fn allocation_for_kind(component: &MealPlanComponent, full: bool) -> ConsumedAmount {
    if full {
        return component.amount;
    }
    match component.amount {
        ConsumedAmount::Servings(_) => ConsumedAmount::Servings(Decimal::ONE),
        other => other,
    }
}

pub fn build_participant(
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

pub fn merge_participant(
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

pub fn validate_guest_groups(
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

pub fn merge_guest_group(
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

pub fn sync_allocations(
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

pub fn participant_status_to_meal(
    status: ParticipantStatus,
    assumption: Assumption,
) -> MealPlanStatus {
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

pub fn has_explicit_allocations(participants: &[NewMealParticipant]) -> bool {
    participants
        .iter()
        .any(|participant| !participant.allocations.is_empty())
}

pub fn apply_equal_portioning(entry: &mut MealPlanEntry) {
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

pub fn require_household_attendance<P, G>(
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

pub fn set_allocation(
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

pub fn validate_participants(
    participants: &[NewMealParticipant],
    components: &[MealPlanComponent],
) -> crate::error::Result<()> {
    let mut errors = ValidationErrors::new();
    if participants.is_empty() {
        errors.push("participants", "A meal needs at least one participant");
    }

    let mut seen_members = std::collections::HashSet::new();
    for (index, participant) in participants.iter().enumerate() {
        if !seen_members.insert(participant.member_id) {
            errors.push(
                format!("participants.{index}.member"),
                "This member is already a participant",
            );
        }

        let mut seen_components = std::collections::HashSet::new();
        for (alloc_index, allocation) in participant.allocations.iter().enumerate() {
            let field = format!("participants.{index}.allocations.{alloc_index}");
            if !seen_components.insert(allocation.component_id) {
                errors.push(&field, "This component is allocated twice");
                continue;
            }
            let Some(component) = components
                .iter()
                .find(|component| component.id == allocation.component_id)
            else {
                errors.push(&field, "Unknown component");
                continue;
            };
            if allocation.allocated.value() <= Decimal::ZERO {
                errors.push(format!("{field}.amount"), "Must be more than zero");
            }
            if allocation.allocated.kind_code() != component.amount.kind_code() {
                errors.push(
                    format!("{field}.amount"),
                    "Allocation must match the component's amount kind",
                );
            }
        }
    }

    errors.into_result()
}
