use std::fmt;
use std::str::FromStr;

use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

use super::{
    ConsumedAmount, HouseholdMemberId, MealPlanComponentId, MealPlanEntryId, NutritionFacts,
    NutritionQuality, ProductId, Revision, UserId,
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
    pub product_name: String,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MealPlanComponent {
    pub id: MealPlanComponentId,
    pub product_id: ProductId,
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
    pub member_id: HouseholdMemberId,
    pub planned_on: Date,
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub status: MealPlanStatus,
    pub components: Vec<MealPlanComponent>,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub resolved_by: Option<UserId>,
    pub resolved_at: Option<OffsetDateTime>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewMealPlanComponent {
    pub id: Option<MealPlanComponentId>,
    pub product_id: ProductId,
    pub amount: ConsumedAmount,
}

#[derive(Debug, Clone)]
pub struct NewMealPlanEntry {
    pub id: Option<MealPlanEntryId>,
    pub member_id: HouseholdMemberId,
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

#[derive(Debug, Clone)]
pub struct ConfirmMealPlanEntry {
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub components: Vec<ActualMealPlanComponent>,
    pub actor_id: UserId,
}

#[derive(Debug, Clone)]
pub struct ConfirmMealPlanComponent {
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub amount: ConsumedAmount,
    pub actor_id: UserId,
}

pub fn validate_components(components: &[NewMealPlanComponent]) -> crate::error::Result<()> {
    let mut errors = ValidationErrors::new();
    if components.is_empty() {
        errors.push("components", "Add at least one product");
    }
    for (index, component) in components.iter().enumerate() {
        if component.amount.value() <= rust_decimal::Decimal::ZERO {
            errors.push(
                format!("components.{index}.amount"),
                "Must be more than zero",
            );
        }
    }
    errors.into_result()
}

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
