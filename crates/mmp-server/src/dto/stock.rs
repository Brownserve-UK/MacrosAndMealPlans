use mmp_core::domain::{
    Availability, Confidence, NewStockItem, Patch, ProductAvailability, SourceDate, SourceDateKind,
    StockEvent, StockItem, StockItemId, StockItemPatch, StockLevel, StorageLocation, TrackingMode,
    Unit, UsabilityDeadline,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::{PageMeta, QuantityDto, SortDirectionDto, iso_date};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackingModeDto {
    Exact,
    Estimated,
    NotTracked,
}

impl From<TrackingMode> for TrackingModeDto {
    fn from(value: TrackingMode) -> Self {
        match value {
            TrackingMode::Exact => Self::Exact,
            TrackingMode::Estimated => Self::Estimated,
            TrackingMode::NotTracked => Self::NotTracked,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageLocationDto {
    Ambient,
    Chilled,
    Frozen,
}

impl From<StorageLocation> for StorageLocationDto {
    fn from(value: StorageLocation) -> Self {
        match value {
            StorageLocation::Ambient => Self::Ambient,
            StorageLocation::Chilled => Self::Chilled,
            StorageLocation::Frozen => Self::Frozen,
        }
    }
}

impl From<StorageLocationDto> for StorageLocation {
    fn from(value: StorageLocationDto) -> Self {
        match value {
            StorageLocationDto::Ambient => Self::Ambient,
            StorageLocationDto::Chilled => Self::Chilled,
            StorageLocationDto::Frozen => Self::Frozen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceDateKindDto {
    UseBy,
    BestBefore,
}

impl From<SourceDateKind> for SourceDateKindDto {
    fn from(value: SourceDateKind) -> Self {
        match value {
            SourceDateKind::UseBy => Self::UseBy,
            SourceDateKind::BestBefore => Self::BestBefore,
        }
    }
}

impl From<SourceDateKindDto> for SourceDateKind {
    fn from(value: SourceDateKindDto) -> Self {
        match value {
            SourceDateKindDto::UseBy => Self::UseBy,
            SourceDateKindDto::BestBefore => Self::BestBefore,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum StockLevelDto {
    Exact {
        quantity: QuantityDto,
    },
    Estimated {
        #[serde(with = "rust_decimal::serde::float")]
        #[schema(value_type = f64)]
        low: Decimal,
        #[serde(with = "rust_decimal::serde::float")]
        #[schema(value_type = f64)]
        high: Decimal,
        unit: Unit,
    },
    NotTracked,
}

impl From<StockLevel> for StockLevelDto {
    fn from(value: StockLevel) -> Self {
        match value {
            StockLevel::Exact { quantity } => Self::Exact {
                quantity: quantity.into(),
            },
            StockLevel::Estimated { low, high, unit } => Self::Estimated { low, high, unit },
            StockLevel::NotTracked => Self::NotTracked,
        }
    }
}

impl From<StockLevelDto> for StockLevel {
    fn from(value: StockLevelDto) -> Self {
        match value {
            StockLevelDto::Exact { quantity } => Self::Exact {
                quantity: quantity.into(),
            },
            StockLevelDto::Estimated { low, high, unit } => Self::Estimated { low, high, unit },
            StockLevelDto::NotTracked => Self::NotTracked,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceDateDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-25")]
    pub date: Date,
    pub kind: SourceDateKindDto,
}

impl From<SourceDate> for SourceDateDto {
    fn from(value: SourceDate) -> Self {
        Self {
            date: value.date,
            kind: value.kind.into(),
        }
    }
}

impl From<SourceDateDto> for SourceDate {
    fn from(value: SourceDateDto) -> Self {
        Self {
            date: value.date,
            kind: value.kind.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsabilityDeadlineDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-30")]
    pub date: Date,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
}

impl From<UsabilityDeadline> for UsabilityDeadlineDto {
    fn from(value: UsabilityDeadline) -> Self {
        Self {
            date: value.date,
            basis: value.basis,
        }
    }
}

impl From<UsabilityDeadlineDto> for UsabilityDeadline {
    fn from(value: UsabilityDeadlineDto) -> Self {
        Self {
            date: value.date,
            basis: value.basis,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StockItemDto {
    pub id: Uuid,
    pub product_id: Uuid,
    pub tracking_mode: TrackingModeDto,
    pub level: StockLevelDto,
    pub storage_location: StorageLocationDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_date: Option<SourceDateDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usability_deadline: Option<UsabilityDeadlineDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub archived_at: Option<OffsetDateTime>,
}

impl From<StockItem> for StockItemDto {
    fn from(value: StockItem) -> Self {
        Self {
            id: value.id.as_uuid(),
            product_id: value.product_id.as_uuid(),
            tracking_mode: value.tracking_mode().into(),
            level: value.level.into(),
            storage_location: value.storage_location.into(),
            source_date: value.source_date.map(Into::into),
            usability_deadline: value.usability_deadline.map(Into::into),
            note: value.note,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateStockItemRequest {
    pub product_id: Uuid,
    pub level: StockLevelDto,
    pub storage_location: StorageLocationDto,
    #[serde(default)]
    pub source_date: Option<SourceDateDto>,
    #[serde(default)]
    pub usability_deadline: Option<UsabilityDeadlineDto>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub subject_member_id: Option<Uuid>,
}

impl CreateStockItemRequest {
    pub fn into_domain(self) -> NewStockItem {
        NewStockItem {
            product_id: mmp_core::domain::ProductId::from(self.product_id),
            level: self.level.into(),
            storage_location: self.storage_location.into(),
            source_date: self.source_date.map(Into::into),
            usability_deadline: self.usability_deadline.map(Into::into),
            note: self.note,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateStockItemRequest {
    #[serde(default)]
    pub level: Option<StockLevelDto>,
    #[serde(default)]
    pub storage_location: Option<StorageLocationDto>,
    #[serde(default)]
    #[schema(value_type = Option<SourceDateDto>)]
    pub source_date: Patch<SourceDateDto>,
    #[serde(default)]
    #[schema(value_type = Option<UsabilityDeadlineDto>)]
    pub usability_deadline: Patch<UsabilityDeadlineDto>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub note: Patch<String>,
    #[serde(default)]
    pub subject_member_id: Option<Uuid>,
}

impl From<UpdateStockItemRequest> for StockItemPatch {
    fn from(value: UpdateStockItemRequest) -> Self {
        StockItemPatch {
            level: value.level.map(Into::into),
            storage_location: value.storage_location.map(Into::into),
            source_date: value.source_date.map(Into::into),
            usability_deadline: value.usability_deadline.map(Into::into),
            note: value.note,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StockEventDto {
    pub id: Uuid,
    pub stock_item_id: Uuid,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity_delta: Option<QuantityDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_member_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub occurred_at: OffsetDateTime,
}

impl From<StockEvent> for StockEventDto {
    fn from(value: StockEvent) -> Self {
        Self {
            id: value.id.as_uuid(),
            stock_item_id: value.stock_item_id.as_uuid(),
            kind: value.kind.code().to_owned(),
            quantity_delta: value.quantity_delta.map(Into::into),
            actor_user_id: value.actor_user_id.map(|id| id.as_uuid()),
            subject_member_id: value.subject_member_id.map(|id| id.as_uuid()),
            note: value.note,
            occurred_at: value.occurred_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AvailabilityDto {
    Quantified {
        on_hand: QuantityDto,
        planned_demand: QuantityDto,
        unallocated: QuantityDto,
        confidence: ConfidenceDto,
    },
    AssumedAvailable,
    Unknown,
    Absent,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceDto {
    Exact,
    Estimated,
}

impl From<Confidence> for ConfidenceDto {
    fn from(value: Confidence) -> Self {
        match value {
            Confidence::Exact => Self::Exact,
            Confidence::Estimated => Self::Estimated,
        }
    }
}

impl From<Availability> for AvailabilityDto {
    fn from(value: Availability) -> Self {
        match value {
            Availability::Quantified {
                on_hand,
                planned_demand,
                unallocated,
                confidence,
            } => Self::Quantified {
                on_hand: on_hand.into(),
                planned_demand: planned_demand.into(),
                unallocated: unallocated.into(),
                confidence: confidence.into(),
            },
            Availability::AssumedAvailable => Self::AssumedAvailable,
            Availability::Unknown => Self::Unknown,
            Availability::Absent => Self::Absent,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProductAvailabilityDto {
    pub product_id: Uuid,
    pub availability: AvailabilityDto,
    pub demand_incomplete: bool,
}

impl From<ProductAvailability> for ProductAvailabilityDto {
    fn from(value: ProductAvailability) -> Self {
        Self {
            product_id: value.product_id.as_uuid(),
            availability: value.availability.into(),
            demand_incomplete: value.demand_incomplete,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StockPage {
    pub items: Vec<StockItemDto>,
    pub page: PageMeta,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StockListQuery {
    pub product_id: Option<Uuid>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<SortDirectionDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StockAvailabilityQuery {
    pub product_id: Option<Uuid>,
    #[serde(with = "iso_date::option", default)]
    #[param(value_type = Option<String>, example = "2026-08-25")]
    pub from: Option<Date>,
    #[serde(with = "iso_date::option", default)]
    #[param(value_type = Option<String>, example = "2026-09-08")]
    pub to: Option<Date>,
}

pub fn stock_item_id(id: Uuid) -> StockItemId {
    StockItemId::from(id)
}

#[allow(dead_code)]
fn to_decimal(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or(Decimal::ZERO)
}
