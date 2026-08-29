use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::{
    HouseholdMemberId, Patch, ProductId, Quantity, Revision, StockEventId, StockItemId, Unit,
    UserId,
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
    Discarded,
    Corrected,
    Observed,
    Moved,
    ModeChanged,
    Archived,
}

impl StockEventKind {
    pub const ALL: [StockEventKind; 8] = [
        StockEventKind::Added,
        StockEventKind::Consumed,
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
    pub note: Option<String>,
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewStockEvent {
    pub kind: StockEventKind,
    pub quantity_delta: Option<Quantity>,
    pub actor_user_id: Option<UserId>,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub note: Option<String>,
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

#[cfg(test)]
#[path = "stock_tests.rs"]
mod tests;
