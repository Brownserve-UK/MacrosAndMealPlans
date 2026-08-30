use std::fmt;
use std::str::FromStr;

use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

use rust_decimal::Decimal;

use super::{
    ConsumedAmount, ConsumptionRecordId, HouseholdMemberId, MealItemRef,
    MealParticipantAllocationId, MealParticipantId, MealPlanComponentId, MealPlanEntryId,
    NutritionFacts, NutritionQuality, Quantity, Revision, UserId,
};
use crate::error::ValidationErrors;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MealSlot {
    Breakfast,
    Lunch,
    Dinner,
    Snacks,
}

impl MealSlot {
    pub const ALL: [MealSlot; 4] = [
        MealSlot::Breakfast,
        MealSlot::Lunch,
        MealSlot::Dinner,
        MealSlot::Snacks,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            MealSlot::Breakfast => "breakfast",
            MealSlot::Lunch => "lunch",
            MealSlot::Dinner => "dinner",
            MealSlot::Snacks => "snacks",
        }
    }

    pub const fn order(self) -> u8 {
        match self {
            MealSlot::Breakfast => 0,
            MealSlot::Lunch => 1,
            MealSlot::Dinner => 2,
            MealSlot::Snacks => 3,
        }
    }

    pub const fn allows_planned_time(self) -> bool {
        !matches!(self, MealSlot::Snacks)
    }
}

impl fmt::Display for MealSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known meal slot")]
pub struct UnknownMealSlot(pub String);

impl FromStr for MealSlot {
    type Err = UnknownMealSlot;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|slot| slot.code() == value)
            .ok_or_else(|| UnknownMealSlot(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MealPlanStatus {
    Planned,
    PartiallyResolved,
    Eaten,
    NotEaten,
}

impl MealPlanStatus {
    pub const ALL: [MealPlanStatus; 4] = [
        MealPlanStatus::Planned,
        MealPlanStatus::PartiallyResolved,
        MealPlanStatus::Eaten,
        MealPlanStatus::NotEaten,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            MealPlanStatus::Planned => "planned",
            MealPlanStatus::PartiallyResolved => "partially_resolved",
            MealPlanStatus::Eaten => "eaten",
            MealPlanStatus::NotEaten => "not_eaten",
        }
    }
}

impl fmt::Display for MealPlanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known meal plan status")]
pub struct UnknownMealPlanStatus(pub String);

impl FromStr for MealPlanStatus {
    type Err = UnknownMealPlanStatus;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.code() == value)
            .ok_or_else(|| UnknownMealPlanStatus(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealPlanComponentSnapshot {
    pub item_name: String,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealPlanComponent {
    pub id: MealPlanComponentId,
    pub item: MealItemRef,
    pub amount: ConsumedAmount,
    pub position: i32,
    pub snapshot: Option<MealPlanComponentSnapshot>,
    pub status: MealPlanStatus,
    pub resolved_by: Option<UserId>,
    pub resolved_at: Option<OffsetDateTime>,
    pub revision: Revision,
    pub display_order: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealPlanEntry {
    pub id: MealPlanEntryId,
    pub scope: MealPlanScope,
    pub member_id: Option<HouseholdMemberId>,
    pub planned_on: Date,
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub status: MealPlanStatus,
    pub components: Vec<MealPlanComponent>,
    pub participants: Vec<MealParticipant>,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub resolved_by: Option<UserId>,
    pub resolved_at: Option<OffsetDateTime>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MealPlanEntry {
    pub fn participant_for(&self, member_id: HouseholdMemberId) -> Option<&MealParticipant> {
        self.participants
            .iter()
            .find(|participant| participant.member_id == member_id)
    }

    pub fn subject_or_owner(
        &self,
        requested: Option<HouseholdMemberId>,
    ) -> Option<HouseholdMemberId> {
        requested
            .or(self.member_id)
            .or_else(|| self.participants.first().map(|p| p.member_id))
    }
}

#[derive(Debug, Clone)]
pub struct NewMealPlanComponent {
    pub id: Option<MealPlanComponentId>,
    pub item: MealItemRef,
    pub amount: ConsumedAmount,
}

#[derive(Debug, Clone)]
pub struct NewMealPlanEntry {
    pub id: Option<MealPlanEntryId>,
    pub scope: MealPlanScope,
    pub member_id: Option<HouseholdMemberId>,
    pub planned_on: Date,
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub components: Vec<NewMealPlanComponent>,
    pub actor_id: UserId,
}

#[derive(Debug, Clone, Default)]
pub struct MealPlanEntryPatch {
    pub planned_on: Option<Date>,
    pub planned_time: Option<Option<Time>>,
    pub slot: Option<MealSlot>,
    pub components: Option<Vec<NewMealPlanComponent>>,
}

#[derive(Debug, Clone)]
pub struct ActualMealPlanComponent {
    pub component_id: MealPlanComponentId,
    pub amount: ConsumedAmount,
}

#[derive(Debug, Clone, Copy)]
pub struct OutcomeActor {
    pub actor_id: UserId,
    pub subject_member_id: Option<HouseholdMemberId>,
}

impl OutcomeActor {
    pub fn own(actor_id: UserId) -> Self {
        Self {
            actor_id,
            subject_member_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfirmMealPlanEntry {
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub components: Vec<ActualMealPlanComponent>,
    pub actor_id: UserId,
    pub subject_member_id: Option<HouseholdMemberId>,
}

#[derive(Debug, Clone)]
pub struct ConfirmMealPlanComponent {
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub amount: ConsumedAmount,
    pub actor_id: UserId,
    pub subject_member_id: Option<HouseholdMemberId>,
}

#[derive(Debug, Clone)]
pub struct SetMealParticipants {
    pub participants: Vec<NewMealParticipant>,
    pub actor_id: UserId,
}

pub fn validate_components(components: &[NewMealPlanComponent]) -> crate::error::Result<()> {
    let mut errors = ValidationErrors::new();
    if components.is_empty() {
        errors.push("components", "Add at least one item");
    }
    for (index, component) in components.iter().enumerate() {
        if component.amount.value() <= rust_decimal::Decimal::ZERO {
            errors.push(
                format!("components.{index}.amount"),
                "Must be more than zero",
            );
        }
        super::consumption::validate_recipe_amount(
            &format!("components.{index}.amount"),
            component.item,
            &component.amount,
            &mut errors,
        );
    }
    errors.into_result()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MealPlanScope {
    Member,
    Household,
}

impl MealPlanScope {
    pub const ALL: [MealPlanScope; 2] = [MealPlanScope::Member, MealPlanScope::Household];

    pub const fn code(self) -> &'static str {
        match self {
            MealPlanScope::Member => "member",
            MealPlanScope::Household => "household",
        }
    }
}

impl fmt::Display for MealPlanScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known meal plan scope")]
pub struct UnknownMealPlanScope(pub String);

impl FromStr for MealPlanScope {
    type Err = UnknownMealPlanScope;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|scope| scope.code() == value)
            .ok_or_else(|| UnknownMealPlanScope(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ParticipantStatus {
    Planned,
    Eaten,
    NotEaten,
}

impl ParticipantStatus {
    pub const ALL: [ParticipantStatus; 3] = [
        ParticipantStatus::Planned,
        ParticipantStatus::Eaten,
        ParticipantStatus::NotEaten,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            ParticipantStatus::Planned => "planned",
            ParticipantStatus::Eaten => "eaten",
            ParticipantStatus::NotEaten => "not_eaten",
        }
    }

    pub const fn is_resolved(self) -> bool {
        !matches!(self, ParticipantStatus::Planned)
    }
}

impl fmt::Display for ParticipantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known participant status")]
pub struct UnknownParticipantStatus(pub String);

impl FromStr for ParticipantStatus {
    type Err = UnknownParticipantStatus;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.code() == value)
            .ok_or_else(|| UnknownParticipantStatus(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealParticipantAllocation {
    pub id: MealParticipantAllocationId,
    pub component_id: MealPlanComponentId,
    pub allocated: ConsumedAmount,
    pub status: ParticipantStatus,
    pub consumption_record_id: Option<ConsumptionRecordId>,
    pub resolved_by: Option<UserId>,
    pub resolved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealParticipant {
    pub id: MealParticipantId,
    pub member_id: HouseholdMemberId,
    pub allocations: Vec<MealParticipantAllocation>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewMealParticipantAllocation {
    pub component_id: MealPlanComponentId,
    pub allocated: ConsumedAmount,
}

#[derive(Debug, Clone)]
pub struct NewMealParticipant {
    pub id: Option<MealParticipantId>,
    pub member_id: HouseholdMemberId,
    pub allocations: Vec<NewMealParticipantAllocation>,
}

#[derive(Debug, Clone)]
pub struct AllocationOutcome {
    pub allocated: ConsumedAmount,
    pub status: ParticipantStatus,
    pub confirmed: Option<ConsumedAmount>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComponentPreparation {
    pub prepared: ConsumedAmount,
    pub allocated: Option<ConsumedAmount>,
    pub unallocated: Option<ConsumedAmount>,
    pub leftover: Option<ConsumedAmount>,
    pub shortage: bool,
}

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

fn roll_up_status(statuses: impl IntoIterator<Item = ParticipantStatus>) -> MealPlanStatus {
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
        MealPlanStatus::Planned
    } else if any_pending {
        MealPlanStatus::PartiallyResolved
    } else if any_eaten {
        MealPlanStatus::Eaten
    } else {
        MealPlanStatus::NotEaten
    }
}

pub fn derive_participant_status(participant: &MealParticipant) -> MealPlanStatus {
    roll_up_status(participant.allocations.iter().map(|a| a.status))
}

pub fn derive_component_status(
    component_id: MealPlanComponentId,
    participants: &[MealParticipant],
) -> MealPlanStatus {
    roll_up_status(
        participants
            .iter()
            .flat_map(|p| p.allocations.iter())
            .filter(|a| a.component_id == component_id)
            .map(|a| a.status),
    )
}

pub fn derive_entry_status(participants: &[MealParticipant]) -> MealPlanStatus {
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

    if !any || (any_pending && !any_resolved) {
        return MealPlanStatus::Planned;
    }
    if any_pending {
        return MealPlanStatus::PartiallyResolved;
    }

    let component_statuses: Vec<MealPlanStatus> = component_ids
        .into_iter()
        .map(|component_id| derive_component_status(component_id, participants))
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

impl MealPlanComponentSnapshot {
    pub fn per_unit(&self, prepared: &ConsumedAmount) -> NutritionFacts {
        let divisor = prepared.value();
        if divisor.is_zero() {
            return self.nutrition.clone();
        }
        self.nutrition.scale(Decimal::ONE / divisor)
    }

    pub fn scaled_to(
        &self,
        prepared: &ConsumedAmount,
        confirmed: &ConsumedAmount,
    ) -> NutritionFacts {
        NutritionFacts {
            basis: None,
            ..self.per_unit(prepared).scale(confirmed.value())
        }
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

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
