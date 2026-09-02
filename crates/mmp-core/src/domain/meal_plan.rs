use std::fmt;
use std::str::FromStr;

use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

use rust_decimal::Decimal;

use super::{
    ConsumedAmount, ConsumptionRecordId, HouseholdMemberId, MealGuestAllocationId,
    MealGuestGroupId, MealItemRef, MealParticipantAllocationId, MealParticipantId,
    MealPlanComponentId, MealPlanEntryId, MealTimes, NutritionFacts, NutritionQuality, Quantity,
    Revision, UserId,
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
    Assumed,
    PartiallyResolved,
    Eaten,
    NotEaten,
}

impl MealPlanStatus {
    pub const ALL: [MealPlanStatus; 5] = [
        MealPlanStatus::Planned,
        MealPlanStatus::Assumed,
        MealPlanStatus::PartiallyResolved,
        MealPlanStatus::Eaten,
        MealPlanStatus::NotEaten,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            MealPlanStatus::Planned => "planned",
            MealPlanStatus::Assumed => "assumed",
            MealPlanStatus::PartiallyResolved => "partially_resolved",
            MealPlanStatus::Eaten => "eaten",
            MealPlanStatus::NotEaten => "not_eaten",
        }
    }

    pub const fn is_unresolved(self) -> bool {
        matches!(self, MealPlanStatus::Planned | MealPlanStatus::Assumed)
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
    pub revision: Revision,
    pub display_order: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum Portioning {
    Equal,
    Custom,
}

impl Portioning {
    pub const ALL: [Portioning; 2] = [Portioning::Equal, Portioning::Custom];

    pub const fn code(self) -> &'static str {
        match self {
            Portioning::Equal => "equal",
            Portioning::Custom => "custom",
        }
    }
}

impl fmt::Display for Portioning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known portioning mode")]
pub struct UnknownPortioning(pub String);

impl FromStr for Portioning {
    type Err = UnknownPortioning;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.code() == value)
            .ok_or_else(|| UnknownPortioning(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealOptOut {
    pub member_id: HouseholdMemberId,
    pub created_by: UserId,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum SlotAttendance {
    Participating,
    OptedOut,
    SelfCatering,
    Available,
}

impl SlotAttendance {
    pub const fn code(self) -> &'static str {
        match self {
            SlotAttendance::Participating => "participating",
            SlotAttendance::OptedOut => "opted_out",
            SlotAttendance::SelfCatering => "self_catering",
            SlotAttendance::Available => "available",
        }
    }
}

impl fmt::Display for SlotAttendance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Assumption {
    pub assumed: bool,
}

impl Assumption {
    pub const NONE: Assumption = Assumption { assumed: false };

    pub const fn new(assumed: bool) -> Self {
        Self { assumed }
    }

    pub fn for_entry(
        entry: &MealPlanEntry,
        now: OffsetDateTime,
        meal_times: &MealTimes,
        enabled: bool,
    ) -> Self {
        Self::for_occurrence(
            entry.planned_on,
            entry.planned_time,
            entry.slot,
            now,
            meal_times,
            enabled,
        )
    }

    pub fn for_occurrence(
        planned_on: Date,
        planned_time: Option<Time>,
        slot: MealSlot,
        now: OffsetDateTime,
        meal_times: &MealTimes,
        enabled: bool,
    ) -> Self {
        if !enabled {
            return Self::NONE;
        }
        let at = planned_time
            .or_else(|| meal_times.for_slot(slot))
            .unwrap_or(Time::MAX);
        Self::new(planned_on.with_time(at).assume_utc() <= now)
    }

    fn pending(self) -> MealPlanStatus {
        if self.assumed {
            MealPlanStatus::Assumed
        } else {
            MealPlanStatus::Planned
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssumptionRules {
    pub now: OffsetDateTime,
    pub meal_times: MealTimes,
    pub enabled: bool,
}

impl AssumptionRules {
    pub fn disabled(now: OffsetDateTime, meal_times: MealTimes) -> Self {
        Self {
            now,
            meal_times,
            enabled: false,
        }
    }

    pub fn for_entry(&self, entry: &MealPlanEntry) -> Assumption {
        Assumption::for_entry(entry, self.now, &self.meal_times, self.enabled)
    }

    pub fn for_occurrence(
        &self,
        planned_on: Date,
        planned_time: Option<Time>,
        slot: MealSlot,
    ) -> Assumption {
        Assumption::for_occurrence(
            planned_on,
            planned_time,
            slot,
            self.now,
            &self.meal_times,
            self.enabled,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealPlanEntry {
    pub id: MealPlanEntryId,
    pub scope: MealPlanScope,
    pub member_id: Option<HouseholdMemberId>,
    pub planned_on: Date,
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub portioning: Portioning,
    pub components: Vec<MealPlanComponent>,
    pub participants: Vec<MealParticipant>,
    pub guest_groups: Vec<MealGuestGroup>,
    pub opted_out: Vec<MealOptOut>,
    pub created_by: UserId,
    pub updated_by: UserId,
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

    pub fn has_opted_out(&self, member_id: HouseholdMemberId) -> bool {
        self.opted_out
            .iter()
            .any(|opt_out| opt_out.member_id == member_id)
    }

    pub fn status(&self, assumption: Assumption) -> MealPlanStatus {
        derive_entry_status(&self.participants, &self.guest_groups, assumption)
    }

    pub fn component_status(
        &self,
        component_id: MealPlanComponentId,
        assumption: Assumption,
    ) -> MealPlanStatus {
        derive_component_status(
            component_id,
            &self.participants,
            &self.guest_groups,
            assumption,
        )
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
    pub portioning: Portioning,
    pub components: Vec<NewMealPlanComponent>,
    pub participants: Option<Vec<NewMealParticipant>>,
    pub guest_groups: Vec<NewMealGuestGroup>,
    pub actor_id: UserId,
}

#[derive(Debug, Clone, Default)]
pub struct MealPlanEntryPatch {
    pub planned_on: Option<Date>,
    pub planned_time: Option<Option<Time>>,
    pub slot: Option<MealSlot>,
    pub portioning: Option<Portioning>,
    pub components: Option<Vec<NewMealPlanComponent>>,
    pub participants: Option<Vec<NewMealParticipant>>,
    pub guest_groups: Option<Vec<NewMealGuestGroup>>,
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
pub struct ReplacementItem {
    pub item: MealItemRef,
    pub amount: ConsumedAmount,
}

#[derive(Debug, Clone, Default)]
pub struct ChangedMealOutcome {
    pub components: Vec<ActualMealPlanComponent>,
    pub replacements: Vec<ReplacementItem>,
}

impl ChangedMealOutcome {
    pub fn is_empty(&self) -> bool {
        self.components.is_empty() && self.replacements.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum ReviewedMealOutcome {
    AsPlanned,
    NotEaten,
    Changed(ChangedMealOutcome),
}

#[derive(Debug, Clone)]
pub struct ReviewedMemberOutcome {
    pub member_id: HouseholdMemberId,
    pub outcome: ReviewedMealOutcome,
}

#[derive(Debug, Clone)]
pub struct ReviewedGuestOutcome {
    pub source_group_id: MealGuestGroupId,
    pub count: i32,
    pub outcome: ReviewedMealOutcome,
}

#[derive(Debug, Clone)]
pub struct ReviewMealOutcomes {
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub members: Vec<ReviewedMemberOutcome>,
    pub guests: Vec<ReviewedGuestOutcome>,
    pub actor_id: UserId,
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
    pub guest_groups: Vec<NewMealGuestGroup>,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealGuestAllocation {
    pub id: MealGuestAllocationId,
    pub component_id: MealPlanComponentId,
    pub allocated: ConsumedAmount,
    pub status: ParticipantStatus,
    pub confirmed: Option<ConsumedAmount>,
    pub resolved_by: Option<UserId>,
    pub resolved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealGuestGroup {
    pub id: MealGuestGroupId,
    pub count: i32,
    pub allocations: Vec<MealGuestAllocation>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewMealGuestAllocation {
    pub component_id: MealPlanComponentId,
    pub allocated: ConsumedAmount,
}

#[derive(Debug, Clone)]
pub struct NewMealGuestGroup {
    pub id: Option<MealGuestGroupId>,
    pub count: i32,
    pub allocations: Vec<NewMealGuestAllocation>,
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
