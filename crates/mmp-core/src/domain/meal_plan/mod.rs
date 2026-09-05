use crate::domain::str_enum::str_enum;
use std::fmt;

use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

use rust_decimal::Decimal;

use super::{
    ConsumedAmount, ConsumptionRecordId, HouseholdMemberId, MealGuestAllocationId,
    MealGuestGroupId, MealItemRef, MealParticipantAllocationId, MealParticipantId,
    MealPlanComponentId, MealPlanEntryId, MealTimes, NutritionFacts, NutritionQuality, Revision,
    UserId,
};

str_enum!(MealPlanScope, UnknownMealPlanScope, "meal plan scope");
str_enum!(MealPlanStatus, UnknownMealPlanStatus, "meal plan status");
str_enum!(MealSlot, UnknownMealSlot, "meal slot");
str_enum!(
    ParticipantStatus,
    UnknownParticipantStatus,
    "participant status"
);
str_enum!(Portioning, UnknownPortioning, "portioning mode");

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

mod components;
mod outcomes;
mod participation;
mod status;

pub use components::{make_components, merge_components, validate_components};
pub use outcomes::{
    actual_components_for_member, build_guest_results, component_still_eaten, find_component,
    outcomes_for_component, pending_component_ids, replacements_for, require_allocation_planned,
    require_editable, require_planned, require_subject_pending, validate_actual_components,
};
pub use participation::{
    apply_equal_portioning, build_participant, has_explicit_allocations, merge_guest_group,
    merge_participant, participant_status_to_meal, require_household_attendance, set_allocation,
    sync_allocations, validate_guest_groups, validate_participants,
};
pub use status::{
    allocated_total, derive_component_status, derive_entry_status, derive_guest_status,
    derive_participant_status, effective_consumption, equal_split, preparation_for,
};

pub(crate) const MEAL_PLAN_ENTRY: &str = "meal plan entry";
pub(crate) const MEAL_PLAN_COMPONENT: &str = "meal plan component";

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
