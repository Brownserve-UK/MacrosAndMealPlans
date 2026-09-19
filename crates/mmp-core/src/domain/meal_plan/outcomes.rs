use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::{
    AllocationOutcome, Assumption, ConfirmMealPlanEntry, MEAL_PLAN_COMPONENT, MealGuestAllocation,
    MealGuestGroup, MealPlanComponent, MealPlanEntry, MealPlanStatus, ParticipantStatus,
    ReplacementItem, ReviewMealOutcomes, ReviewedMealOutcome,
};
use crate::domain::{
    ConsumedAmount, ConsumptionRecord, HouseholdMemberId, MealPlanComponentId, Revision,
};
use crate::error::{CoreError, Result, ValidationErrors};

pub fn outcomes_for_component(
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

pub fn actual_components_for_member(
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

pub fn replacements_for(outcome: &ReviewedMealOutcome) -> &[ReplacementItem] {
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

pub fn build_guest_results(
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
        let preserves_identity = reviewed.len() == 1;
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
                id: if preserves_identity {
                    source.id
                } else {
                    Default::default()
                },
                count: reviewed.count,
                name: source.name.clone(),
                note: source.note.clone(),
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

pub fn pending_component_ids(
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

pub fn component_still_eaten(entry: &MealPlanEntry, component_id: MealPlanComponentId) -> bool {
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

pub fn find_component(
    entry: &MealPlanEntry,
    component_id: MealPlanComponentId,
) -> Result<&MealPlanComponent> {
    entry
        .components
        .iter()
        .find(|component| component.id == component_id)
        .ok_or_else(|| CoreError::not_found(MEAL_PLAN_COMPONENT, component_id))
}

pub fn require_allocation_planned(
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

pub fn require_subject_pending(entry: &MealPlanEntry, member_id: HouseholdMemberId) -> Result<()> {
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

pub fn validate_actual_components(
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

pub fn require_planned(entry: &MealPlanEntry) -> Result<()> {
    if entry.status(Assumption::NONE) == MealPlanStatus::Planned {
        Ok(())
    } else {
        Err(CoreError::conflict(
            "Resolved meal plans cannot be changed.",
        ))
    }
}

pub fn require_editable(entry: &MealPlanEntry) -> Result<()> {
    match entry.status(Assumption::NONE) {
        MealPlanStatus::Planned | MealPlanStatus::Assumed | MealPlanStatus::PartiallyResolved => {
            Ok(())
        }
        MealPlanStatus::Eaten | MealPlanStatus::NotEaten => Err(CoreError::conflict(
            "This meal is fully resolved and cannot be changed.",
        )),
    }
}
