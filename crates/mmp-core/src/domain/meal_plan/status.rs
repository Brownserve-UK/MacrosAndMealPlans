use rust_decimal::Decimal;

use super::{
    AllocationOutcome, Assumption, ComponentPreparation, MealGuestGroup, MealParticipant,
    MealPlanComponentId, MealPlanStatus, ParticipantStatus,
};
use crate::domain::{ConsumedAmount, Quantity};

fn amount_zero_like(reference: &ConsumedAmount) -> ConsumedAmount {
    amount_rebuild(reference, Decimal::ZERO)
}

fn amount_rebuild(reference: &ConsumedAmount, value: Decimal) -> ConsumedAmount {
    match reference {
        ConsumedAmount::Measure(quantity) => {
            ConsumedAmount::Measure(Quantity::new(value, quantity.unit))
        }
        ConsumedAmount::Servings(_) => ConsumedAmount::Servings(value),
        ConsumedAmount::Packs(_) => ConsumedAmount::Packs(value),
    }
}

fn amount_in_kind_of(reference: &ConsumedAmount, other: &ConsumedAmount) -> Option<Decimal> {
    match (reference, other) {
        (ConsumedAmount::Measure(target), ConsumedAmount::Measure(source)) => {
            source.convert_to(target.unit).ok().map(|q| q.amount)
        }
        (ConsumedAmount::Servings(_), ConsumedAmount::Servings(value))
        | (ConsumedAmount::Packs(_), ConsumedAmount::Packs(value)) => Some(*value),
        _ => None,
    }
}

fn amount_add(a: &ConsumedAmount, b: &ConsumedAmount) -> Option<ConsumedAmount> {
    amount_in_kind_of(a, b).map(|value| amount_rebuild(a, a.value() + value))
}

fn amount_sub_floor(a: &ConsumedAmount, b: &ConsumedAmount) -> Option<ConsumedAmount> {
    amount_in_kind_of(a, b).map(|value| amount_rebuild(a, (a.value() - value).max(Decimal::ZERO)))
}

fn amount_greater(a: &ConsumedAmount, b: &ConsumedAmount) -> bool {
    amount_in_kind_of(a, b)
        .map(|value| a.value() > value)
        .unwrap_or(false)
}

pub fn effective_consumption(outcome: &AllocationOutcome) -> ConsumedAmount {
    match outcome.status {
        ParticipantStatus::NotEaten => amount_zero_like(&outcome.allocated),
        ParticipantStatus::Planned => outcome.allocated,
        ParticipantStatus::Eaten => outcome.confirmed.unwrap_or(outcome.allocated),
    }
}

pub fn allocated_total(
    prepared: &ConsumedAmount,
    allocations: &[ConsumedAmount],
) -> Option<ConsumedAmount> {
    let mut total = amount_zero_like(prepared);
    for allocation in allocations {
        total = amount_add(&total, allocation)?;
    }
    Some(total)
}

pub fn forecast_remaining(
    forecast: &ConsumedAmount,
    settled: &[ConsumedAmount],
) -> Option<ConsumedAmount> {
    let total = allocated_total(forecast, settled)?;
    amount_sub_floor(forecast, &total)
}

pub fn preparation_for(
    prepared: &ConsumedAmount,
    outcomes: &[AllocationOutcome],
) -> ComponentPreparation {
    let allocations: Vec<ConsumedAmount> = outcomes.iter().map(|o| o.allocated).collect();
    let allocated = allocated_total(prepared, &allocations);
    let unallocated = allocated
        .as_ref()
        .and_then(|total| amount_sub_floor(prepared, total));

    let mut consumed = Some(amount_zero_like(prepared));
    for outcome in outcomes {
        consumed = consumed
            .as_ref()
            .and_then(|running| amount_add(running, &effective_consumption(outcome)));
    }
    let leftover = consumed
        .as_ref()
        .and_then(|total| amount_sub_floor(prepared, total));

    let shortage = allocated
        .as_ref()
        .map(|total| amount_greater(total, prepared))
        .unwrap_or(false);

    ComponentPreparation {
        prepared: *prepared,
        allocated,
        unallocated,
        leftover,
        shortage,
    }
}

fn roll_up_status(
    statuses: impl IntoIterator<Item = ParticipantStatus>,
    assumption: Assumption,
) -> MealPlanStatus {
    let mut any = false;
    let mut any_pending = false;
    let mut any_resolved = false;
    let mut any_eaten = false;
    for status in statuses {
        any = true;
        match status {
            ParticipantStatus::Planned => any_pending = true,
            ParticipantStatus::Eaten => {
                any_resolved = true;
                any_eaten = true;
            }
            ParticipantStatus::NotEaten => any_resolved = true,
        }
    }

    if !any || (any_pending && !any_resolved) {
        assumption.pending()
    } else if any_pending {
        MealPlanStatus::PartiallyResolved
    } else if any_eaten {
        MealPlanStatus::Eaten
    } else {
        MealPlanStatus::NotEaten
    }
}

pub fn derive_participant_status(
    participant: &MealParticipant,
    assumption: Assumption,
) -> MealPlanStatus {
    roll_up_status(participant.allocations.iter().map(|a| a.status), assumption)
}

pub fn derive_guest_status(group: &MealGuestGroup, assumption: Assumption) -> MealPlanStatus {
    roll_up_status(
        group.allocations.iter().map(|allocation| allocation.status),
        assumption,
    )
}

pub fn derive_component_status(
    component_id: MealPlanComponentId,
    participants: &[MealParticipant],
    guest_groups: &[MealGuestGroup],
    assumption: Assumption,
) -> MealPlanStatus {
    roll_up_status(
        participants
            .iter()
            .flat_map(|p| p.allocations.iter())
            .filter(|a| a.component_id == component_id)
            .map(|a| a.status)
            .chain(
                guest_groups
                    .iter()
                    .flat_map(|group| group.allocations.iter())
                    .filter(|a| a.component_id == component_id)
                    .map(|a| a.status),
            ),
        assumption,
    )
}

pub fn equal_split(prepared: &ConsumedAmount, shares: usize) -> ConsumedAmount {
    let shares = Decimal::from(shares.max(1) as u64);
    amount_rebuild(prepared, prepared.value() / shares)
}

pub fn derive_entry_status(
    participants: &[MealParticipant],
    guest_groups: &[MealGuestGroup],
    assumption: Assumption,
) -> MealPlanStatus {
    let mut any = false;
    let mut any_pending = false;
    let mut any_resolved = false;
    let mut component_ids: Vec<MealPlanComponentId> = Vec::new();
    for participant in participants {
        for allocation in &participant.allocations {
            any = true;
            if !component_ids.contains(&allocation.component_id) {
                component_ids.push(allocation.component_id);
            }
            match allocation.status {
                ParticipantStatus::Planned => any_pending = true,
                ParticipantStatus::Eaten | ParticipantStatus::NotEaten => any_resolved = true,
            }
        }
    }
    for group in guest_groups {
        for allocation in &group.allocations {
            any = true;
            if !component_ids.contains(&allocation.component_id) {
                component_ids.push(allocation.component_id);
            }
            match allocation.status {
                ParticipantStatus::Planned => any_pending = true,
                ParticipantStatus::Eaten | ParticipantStatus::NotEaten => any_resolved = true,
            }
        }
    }

    if !any || (any_pending && !any_resolved) {
        return assumption.pending();
    }
    if any_pending {
        return MealPlanStatus::PartiallyResolved;
    }

    let component_statuses: Vec<MealPlanStatus> = component_ids
        .into_iter()
        .map(|component_id| {
            roll_up_status(
                participants
                    .iter()
                    .flat_map(|participant| participant.allocations.iter())
                    .filter(|allocation| allocation.component_id == component_id)
                    .map(|allocation| allocation.status)
                    .chain(
                        guest_groups
                            .iter()
                            .flat_map(|group| group.allocations.iter())
                            .filter(|allocation| allocation.component_id == component_id)
                            .map(|allocation| allocation.status),
                    ),
                assumption,
            )
        })
        .collect();
    if component_statuses
        .iter()
        .all(|status| *status == MealPlanStatus::Eaten)
    {
        MealPlanStatus::Eaten
    } else if component_statuses
        .iter()
        .all(|status| *status == MealPlanStatus::NotEaten)
    {
        MealPlanStatus::NotEaten
    } else {
        MealPlanStatus::PartiallyResolved
    }
}
