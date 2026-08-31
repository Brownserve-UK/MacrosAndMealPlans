use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::{
    HouseholdMemberId, Patch, ProductId, Quantity, Revision, StockEffectId, StockEventId,
    StockItemId, Unit, UserId,
};
use crate::error::{Result, ValidationErrors};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
    Exact,
    Estimated,
    NotTracked,
}

impl TrackingMode {
    pub const ALL: [TrackingMode; 3] = [
        TrackingMode::Exact,
        TrackingMode::Estimated,
        TrackingMode::NotTracked,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            TrackingMode::Exact => "exact",
            TrackingMode::Estimated => "estimated",
            TrackingMode::NotTracked => "not_tracked",
        }
    }
}

impl fmt::Display for TrackingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known tracking mode")]
pub struct UnknownTrackingMode(pub String);

impl FromStr for TrackingMode {
    type Err = UnknownTrackingMode;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        TrackingMode::ALL
            .into_iter()
            .find(|m| m.code() == s)
            .ok_or_else(|| UnknownTrackingMode(s.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageLocation {
    Ambient,
    Chilled,
    Frozen,
}

impl StorageLocation {
    pub const ALL: [StorageLocation; 3] = [
        StorageLocation::Ambient,
        StorageLocation::Chilled,
        StorageLocation::Frozen,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            StorageLocation::Ambient => "ambient",
            StorageLocation::Chilled => "chilled",
            StorageLocation::Frozen => "frozen",
        }
    }
}

impl fmt::Display for StorageLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known storage location")]
pub struct UnknownStorageLocation(pub String);

impl FromStr for StorageLocation {
    type Err = UnknownStorageLocation;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        StorageLocation::ALL
            .into_iter()
            .find(|l| l.code() == s)
            .ok_or_else(|| UnknownStorageLocation(s.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDateKind {
    UseBy,
    BestBefore,
}

impl SourceDateKind {
    pub const ALL: [SourceDateKind; 2] = [SourceDateKind::UseBy, SourceDateKind::BestBefore];

    pub const fn code(&self) -> &'static str {
        match self {
            SourceDateKind::UseBy => "use_by",
            SourceDateKind::BestBefore => "best_before",
        }
    }
}

impl fmt::Display for SourceDateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known source date kind")]
pub struct UnknownSourceDateKind(pub String);

impl FromStr for SourceDateKind {
    type Err = UnknownSourceDateKind;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        SourceDateKind::ALL
            .into_iter()
            .find(|k| k.code() == s)
            .ok_or_else(|| UnknownSourceDateKind(s.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceDate {
    pub date: Date,
    pub kind: SourceDateKind,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsabilityDeadline {
    pub date: Date,
    pub basis: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum StockLevel {
    Exact {
        quantity: Quantity,
    },
    Estimated {
        low: Decimal,
        high: Decimal,
        unit: Unit,
    },
    NotTracked,
}

impl StockLevel {
    pub fn tracking_mode(&self) -> TrackingMode {
        match self {
            StockLevel::Exact { .. } => TrackingMode::Exact,
            StockLevel::Estimated { .. } => TrackingMode::Estimated,
            StockLevel::NotTracked => TrackingMode::NotTracked,
        }
    }

    pub fn conservative_quantity(&self) -> Option<Quantity> {
        match self {
            StockLevel::Exact { quantity } => Some(*quantity),
            StockLevel::Estimated { low, unit, .. } => Some(Quantity::new(*low, *unit)),
            StockLevel::NotTracked => None,
        }
    }

    pub fn is_not_tracked(&self) -> bool {
        matches!(self, StockLevel::NotTracked)
    }

    pub fn is_estimated(&self) -> bool {
        matches!(self, StockLevel::Estimated { .. })
    }

    fn validate(&self, errors: &mut ValidationErrors) {
        match self {
            StockLevel::Exact { quantity } => {
                if quantity.amount.is_sign_negative() {
                    errors.push("level.quantity", "Cannot be negative");
                }
            }
            StockLevel::Estimated { low, high, .. } => {
                if low.is_sign_negative() || high.is_sign_negative() {
                    errors.push("level.estimate", "Cannot be negative");
                }
                if low > high {
                    errors.push(
                        "level.estimate",
                        "The low bound cannot exceed the high bound",
                    );
                }
            }
            StockLevel::NotTracked => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockItem {
    pub id: StockItemId,
    pub product_id: ProductId,
    pub level: StockLevel,
    pub storage_location: StorageLocation,
    pub source_date: Option<SourceDate>,
    pub usability_deadline: Option<UsabilityDeadline>,
    pub note: Option<String>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl StockItem {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn tracking_mode(&self) -> TrackingMode {
        self.level.tracking_mode()
    }
}

#[derive(Debug, Clone)]
pub struct NewStockItem {
    pub product_id: ProductId,
    pub level: StockLevel,
    pub storage_location: StorageLocation,
    pub source_date: Option<SourceDate>,
    pub usability_deadline: Option<UsabilityDeadline>,
    pub note: Option<String>,
}

impl NewStockItem {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        self.level.validate(&mut errors);
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default)]
pub struct StockItemPatch {
    pub level: Option<StockLevel>,
    pub storage_location: Option<StorageLocation>,
    pub source_date: Patch<SourceDate>,
    pub usability_deadline: Patch<UsabilityDeadline>,
    pub note: Patch<String>,
}

impl StockItemPatch {
    pub fn is_empty(&self) -> bool {
        self.level.is_none()
            && self.storage_location.is_none()
            && self.source_date.is_unchanged()
            && self.usability_deadline.is_unchanged()
            && self.note.is_unchanged()
    }

    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(level) = &self.level {
            level.validate(&mut errors);
        }
        errors.into_result()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockEventKind {
    Added,
    Consumed,
    Released,
    Discarded,
    Corrected,
    Observed,
    Moved,
    ModeChanged,
    Archived,
}

impl StockEventKind {
    pub const ALL: [StockEventKind; 9] = [
        StockEventKind::Added,
        StockEventKind::Consumed,
        StockEventKind::Released,
        StockEventKind::Discarded,
        StockEventKind::Corrected,
        StockEventKind::Observed,
        StockEventKind::Moved,
        StockEventKind::ModeChanged,
        StockEventKind::Archived,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            StockEventKind::Added => "added",
            StockEventKind::Consumed => "consumed",
            StockEventKind::Released => "released",
            StockEventKind::Discarded => "discarded",
            StockEventKind::Corrected => "corrected",
            StockEventKind::Observed => "observed",
            StockEventKind::Moved => "moved",
            StockEventKind::ModeChanged => "mode_changed",
            StockEventKind::Archived => "archived",
        }
    }
}

impl fmt::Display for StockEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known stock event kind")]
pub struct UnknownStockEventKind(pub String);

impl FromStr for StockEventKind {
    type Err = UnknownStockEventKind;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        StockEventKind::ALL
            .into_iter()
            .find(|k| k.code() == s)
            .ok_or_else(|| UnknownStockEventKind(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockEvent {
    pub id: StockEventId,
    pub stock_item_id: StockItemId,
    pub kind: StockEventKind,
    pub quantity_delta: Option<Quantity>,
    pub actor_user_id: Option<UserId>,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub source: Option<StockEventSource>,
    pub reverses_event_id: Option<StockEventId>,
    pub note: Option<String>,
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewStockEvent {
    pub kind: StockEventKind,
    pub quantity_delta: Option<Quantity>,
    pub actor_user_id: Option<UserId>,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub source: Option<StockEventSource>,
    pub reverses_event_id: Option<StockEventId>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockEventSource {
    pub kind: StockEffectSource,
    pub id: uuid::Uuid,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Exact,
    Estimated,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    Quantified {
        on_hand: Quantity,
        planned_demand: Quantity,
        unallocated: Quantity,
        confidence: Confidence,
    },
    AssumedAvailable,
    Unknown,
    Absent,
}

impl Availability {
    pub fn is_short(&self) -> bool {
        matches!(self, Availability::Quantified { unallocated, .. } if unallocated.amount.is_sign_negative())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductAvailability {
    pub product_id: ProductId,
    pub availability: Availability,
    pub demand_incomplete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockEffectSource {
    MealPlanComponent,
    ConsumptionRecord,
}

impl StockEffectSource {
    pub const ALL: [StockEffectSource; 2] = [
        StockEffectSource::MealPlanComponent,
        StockEffectSource::ConsumptionRecord,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            StockEffectSource::MealPlanComponent => "meal_plan_component",
            StockEffectSource::ConsumptionRecord => "consumption_record",
        }
    }
}

impl fmt::Display for StockEffectSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known stock effect source")]
pub struct UnknownStockEffectSource(pub String);

impl FromStr for StockEffectSource {
    type Err = UnknownStockEffectSource;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        StockEffectSource::ALL
            .into_iter()
            .find(|k| k.code() == s)
            .ok_or_else(|| UnknownStockEffectSource(s.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockEffectState {
    Applied,
    Released,
    ReleaseFailed,
}

impl StockEffectState {
    pub const ALL: [StockEffectState; 3] = [
        StockEffectState::Applied,
        StockEffectState::Released,
        StockEffectState::ReleaseFailed,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            StockEffectState::Applied => "applied",
            StockEffectState::Released => "released",
            StockEffectState::ReleaseFailed => "release_failed",
        }
    }
}

impl fmt::Display for StockEffectState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known stock effect state")]
pub struct UnknownStockEffectState(pub String);

impl FromStr for StockEffectState {
    type Err = UnknownStockEffectState;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        StockEffectState::ALL
            .into_iter()
            .find(|k| k.code() == s)
            .ok_or_else(|| UnknownStockEffectState(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockEffect {
    pub id: StockEffectId,
    pub source_kind: StockEffectSource,
    pub source_id: uuid::Uuid,
    pub stock_item_id: StockItemId,
    pub product_id: ProductId,
    pub state: StockEffectState,
    pub applied_mode: TrackingMode,
    pub applied_unit: Unit,
    pub exact_delta: Option<Decimal>,
    pub low_delta: Option<Decimal>,
    pub high_delta: Option<Decimal>,
    pub requested_value: Decimal,
    pub apply_event_id: StockEventId,
    pub applied_at: OffsetDateTime,
    pub released_at: Option<OffsetDateTime>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewStockEffect {
    pub source_kind: StockEffectSource,
    pub source_id: uuid::Uuid,
    pub stock_item_id: StockItemId,
    pub product_id: ProductId,
    pub applied_mode: TrackingMode,
    pub applied_unit: Unit,
    pub exact_delta: Option<Decimal>,
    pub low_delta: Option<Decimal>,
    pub high_delta: Option<Decimal>,
    pub requested_value: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedTake {
    pub stock_item_id: StockItemId,
    pub requested: Quantity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Shortfall {
    Covered,
    Short {
        amount: Quantity,
        confidence: Confidence,
    },
    Indeterminate {
        amount: Quantity,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeductionPlan {
    NotTracked,
    NoRecord,
    Planned {
        takes: Vec<PlannedTake>,
        shortfall: Shortfall,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedDelta {
    pub new_level: StockLevel,
    pub exact_delta: Option<Decimal>,
    pub low_delta: Option<Decimal>,
    pub high_delta: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleasePlan {
    Restored { new_level: StockLevel },
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockOutcome {
    pub product_id: ProductId,
    pub wanted: Quantity,
    pub deducted: Quantity,
    pub shortfall: Shortfall,
    pub unresolved_release: bool,
}

fn floor_zero(value: Decimal) -> Decimal {
    if value.is_sign_negative() {
        Decimal::ZERO
    } else {
        value
    }
}

fn current_unit(level: &StockLevel) -> Option<Unit> {
    match level {
        StockLevel::Exact { quantity } => Some(quantity.unit),
        StockLevel::Estimated { unit, .. } => Some(*unit),
        StockLevel::NotTracked => None,
    }
}

type FefoKey = (
    bool,
    Option<time::Date>,
    bool,
    Option<time::Date>,
    time::OffsetDateTime,
    uuid::Uuid,
);

fn fefo_key(item: &StockItem) -> FefoKey {
    let deadline = item.usability_deadline.as_ref().map(|d| d.date);
    let source = item.source_date.as_ref().map(|d| d.date);
    (
        deadline.is_none(),
        deadline,
        source.is_none(),
        source,
        item.created_at,
        item.id.as_uuid(),
    )
}

pub fn apply_take(level: &StockLevel, requested: Quantity) -> Option<AppliedDelta> {
    match level {
        StockLevel::Exact { quantity } if quantity.unit == requested.unit => {
            let new = floor_zero(quantity.amount - requested.amount);
            Some(AppliedDelta {
                new_level: StockLevel::Exact {
                    quantity: Quantity::new(new, quantity.unit),
                },
                exact_delta: Some(new - quantity.amount),
                low_delta: None,
                high_delta: None,
            })
        }
        StockLevel::Estimated { low, high, unit } if *unit == requested.unit => {
            let new_low = floor_zero(low - requested.amount);
            let new_high = floor_zero(high - requested.amount);
            Some(AppliedDelta {
                new_level: StockLevel::Estimated {
                    low: new_low,
                    high: new_high,
                    unit: *unit,
                },
                exact_delta: None,
                low_delta: Some(new_low - low),
                high_delta: Some(new_high - high),
            })
        }
        _ => None,
    }
}

pub fn plan_deduction(items: &[StockItem], want: Quantity) -> DeductionPlan {
    let live: Vec<&StockItem> = items.iter().filter(|item| !item.is_archived()).collect();

    if live.iter().any(|item| item.level.is_not_tracked()) {
        return DeductionPlan::NotTracked;
    }
    if live.is_empty() {
        return DeductionPlan::NoRecord;
    }

    let mut candidates = live;
    candidates.sort_by_key(|item| fefo_key(item));

    let mut remaining = want.amount;
    let mut takes = Vec::new();
    let mut skipped_incompatible = false;
    let mut took_from_estimated = false;

    for item in candidates {
        if remaining <= Decimal::ZERO {
            break;
        }
        let Some(unit) = current_unit(&item.level) else {
            continue;
        };
        let Some(available) = item.level.conservative_quantity() else {
            continue;
        };
        let want_here = match Quantity::new(remaining, want.unit).convert_to(unit) {
            Ok(converted) => converted.amount,
            Err(_) => {
                skipped_incompatible = true;
                continue;
            }
        };
        let take_here = want_here.min(available.amount);
        if take_here <= Decimal::ZERO {
            continue;
        }
        if item.level.is_estimated() {
            took_from_estimated = true;
        }
        takes.push(PlannedTake {
            stock_item_id: item.id,
            requested: Quantity::new(take_here, unit),
        });
        let taken_in_want = Quantity::new(take_here, unit)
            .convert_to(want.unit)
            .map(|q| q.amount)
            .unwrap_or(Decimal::ZERO);
        remaining -= taken_in_want;
    }

    let shortfall = if remaining <= Decimal::ZERO {
        Shortfall::Covered
    } else if skipped_incompatible {
        Shortfall::Indeterminate {
            amount: Quantity::new(remaining, want.unit),
        }
    } else {
        Shortfall::Short {
            amount: Quantity::new(remaining, want.unit),
            confidence: if took_from_estimated {
                Confidence::Estimated
            } else {
                Confidence::Exact
            },
        }
    };

    DeductionPlan::Planned { takes, shortfall }
}

pub fn plan_release(item: &StockItem, effect: &StockEffect) -> ReleasePlan {
    let mode_matches = item.tracking_mode() == effect.applied_mode;
    let unit_matches = current_unit(&item.level) == Some(effect.applied_unit);
    if !mode_matches || !unit_matches {
        return ReleasePlan::Failed {
            reason: "The item's tracking has changed since this was applied, so it cannot be put back automatically.".to_owned(),
        };
    }

    match (
        &item.level,
        effect.exact_delta,
        effect.low_delta,
        effect.high_delta,
    ) {
        (StockLevel::Exact { quantity }, Some(delta), _, _) => ReleasePlan::Restored {
            new_level: StockLevel::Exact {
                quantity: Quantity::new(quantity.amount - delta, quantity.unit),
            },
        },
        (StockLevel::Estimated { low, high, unit }, _, Some(low_delta), Some(high_delta)) => {
            ReleasePlan::Restored {
                new_level: StockLevel::Estimated {
                    low: low - low_delta,
                    high: high - high_delta,
                    unit: *unit,
                },
            }
        }
        _ => ReleasePlan::Failed {
            reason: "The stored change does not match the item's current shape.".to_owned(),
        },
    }
}

#[cfg(test)]
#[path = "stock_tests.rs"]
mod tests;
