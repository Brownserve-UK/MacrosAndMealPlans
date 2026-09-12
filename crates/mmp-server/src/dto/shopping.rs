use mmp_core::domain::{
    Assignment, Certainty, DemandSubject, NewPurchase, NewShoppingCadence, NewShoppingListItem,
    OpportunityState, Patch, Purchase, PurchasePatch, PurchaseState, ShoppingCadence,
    ShoppingListItem, ShoppingListItemPatch, ShoppingOpportunity, ShoppingRequirement,
    ShoppingSection, ShoppingTrip, ShoppingTripRow, SuggestionReason, TripState,
    week_day_from_number, week_day_number,
};
use mmp_core::services::{FinishedShop, ShopCount, ShoppingList};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime, Time};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::{PageMeta, QuantityDto, iso_date, iso_time};
use super::stock::{DemandClaimDto, DemandGapDto, DemandSubjectDto};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingCadenceDto {
    #[schema(example = 1, minimum = 1, maximum = 8)]
    pub interval_weeks: u8,
    #[schema(example = json!([3, 6]))]
    pub days: Vec<u8>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-31")]
    pub anchor: Date,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_time::option"
    )]
    #[schema(value_type = Option<String>)]
    pub usual_time: Option<Time>,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<ShoppingCadence> for ShoppingCadenceDto {
    fn from(value: ShoppingCadence) -> Self {
        Self {
            interval_weeks: value.interval_weeks,
            days: value.days.iter().map(week_day_number).collect(),
            anchor: value.anchor,
            usual_time: value.usual_time,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetShoppingCadenceRequest {
    #[schema(example = 1)]
    pub interval_weeks: u8,
    #[schema(example = json!([3, 6]))]
    pub days: Vec<u8>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-31")]
    pub anchor: Date,
    #[serde(default, with = "iso_time::option")]
    #[schema(value_type = Option<String>)]
    pub usual_time: Option<Time>,
}

impl SetShoppingCadenceRequest {
    pub fn into_domain(self) -> Result<NewShoppingCadence, mmp_core::CoreError> {
        let mut days = Vec::with_capacity(self.days.len());
        for number in self.days {
            match week_day_from_number(number) {
                Some(day) => days.push(day),
                None => {
                    let mut errors = mmp_core::error::ValidationErrors::new();
                    errors.push("days", "Use 1 for Monday through to 7 for Sunday.");
                    return Err(errors.into_result().unwrap_err());
                }
            }
        }
        Ok(NewShoppingCadence {
            interval_weeks: self.interval_weeks,
            days,
            anchor: self.anchor,
            usual_time: self.usual_time,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityStateDto {
    Normal,
    Moved,
    OneOff,
}

impl From<OpportunityState> for OpportunityStateDto {
    fn from(value: OpportunityState) -> Self {
        match value {
            OpportunityState::Normal => Self::Normal,
            OpportunityState::Moved => Self::Moved,
            OpportunityState::OneOff => Self::OneOff,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingOpportunityDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-05")]
    pub date: Date,
    pub state: OpportunityStateDto,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date::option"
    )]
    #[schema(value_type = Option<String>, format = Date)]
    pub generated_for: Option<Date>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_time::option"
    )]
    #[schema(value_type = Option<String>)]
    pub usual_time: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub revision: i64,
}

impl From<ShoppingOpportunity> for ShoppingOpportunityDto {
    fn from(value: ShoppingOpportunity) -> Self {
        Self {
            id: value.id.map(|id| id.as_uuid()),
            date: value.date,
            state: value.state.into(),
            generated_for: value.generated_for,
            usual_time: value.usual_time,
            note: value.note,
            revision: value.revision.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionReasonDto {
    UnknownAvailability,
    AssumptionOnly,
    NoProductYet,
}

impl From<SuggestionReason> for SuggestionReasonDto {
    fn from(value: SuggestionReason) -> Self {
        match value {
            SuggestionReason::UnknownAvailability => Self::UnknownAvailability,
            SuggestionReason::AssumptionOnly => Self::AssumptionOnly,
            SuggestionReason::NoProductYet => Self::NoProductYet,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CertaintyDto {
    Definite,
    Suggested { reason: SuggestionReasonDto },
}

impl From<Certainty> for CertaintyDto {
    fn from(value: Certainty) -> Self {
        match value {
            Certainty::Definite => Self::Definite,
            Certainty::Suggested { reason } => Self::Suggested {
                reason: reason.into(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssignmentDto {
    Opportunity {
        #[serde(with = "iso_date")]
        #[schema(value_type = String, format = Date)]
        date: Date,
    },
    NeedsEarlierOpportunity,
    Unassigned,
}

impl From<Assignment> for AssignmentDto {
    fn from(value: Assignment) -> Self {
        match value {
            Assignment::Opportunity { date } => Self::Opportunity { date },
            Assignment::NeedsEarlierOpportunity => Self::NeedsEarlierOpportunity,
            Assignment::Unassigned => Self::Unassigned,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingRequirementDto {
    pub subject: DemandSubjectDto,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<QuantityDto>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date::option"
    )]
    #[schema(value_type = Option<String>, format = Date)]
    pub required_by: Option<Date>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date::option"
    )]
    #[schema(value_type = Option<String>, format = Date)]
    pub use_by_at_least: Option<Date>,
    pub section: ShoppingSection,
    pub certainty: CertaintyDto,
    pub assignment: AssignmentDto,
    pub claims: Vec<DemandClaimDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<DemandGapDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub purchases: Vec<PurchaseDto>,
}

impl From<ShoppingRequirement> for ShoppingRequirementDto {
    fn from(value: ShoppingRequirement) -> Self {
        Self {
            subject: value.subject.into(),
            name: value.name,
            quantity: value.quantity.map(Into::into),
            required_by: value.required_by,
            use_by_at_least: value.use_by_at_least,
            section: value.section,
            certainty: value.certainty.into(),
            assignment: value.assignment.into(),
            claims: value.claims.into_iter().map(Into::into).collect(),
            gaps: value.gaps.into_iter().map(Into::into).collect(),
            purchases: value.purchases.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingListDto {
    pub opportunities: Vec<ShoppingOpportunityDto>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date::option"
    )]
    #[schema(value_type = Option<String>, format = Date)]
    pub focus: Option<Date>,
    pub requirements: Vec<ShoppingRequirementDto>,
    pub manual: Vec<ShoppingListItemDto>,
    pub unplanned: Vec<PurchaseDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip: Option<ShoppingTripDto>,
    pub counts: Vec<ShopCountDto>,
    pub cadence_configured: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PutAwayRequest {
    pub product_id: Uuid,
    pub quantity: QuantityDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<crate::dto::stock::StorageLocationDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShopCountDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-05")]
    pub date: Date,
    pub items: i64,
}

impl From<ShopCount> for ShopCountDto {
    fn from(value: ShopCount) -> Self {
        Self {
            date: value.date,
            items: value.items as i64,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TripStateDto {
    Shopping,
    Finished,
}

impl From<TripState> for TripStateDto {
    fn from(value: TripState) -> Self {
        match value {
            TripState::Shopping => Self::Shopping,
            TripState::Finished => Self::Finished,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingTripRowDto {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingredient_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_meal_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<QuantityDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<ShoppingSection>,
}

impl From<ShoppingTripRow> for ShoppingTripRowDto {
    fn from(value: ShoppingTripRow) -> Self {
        Self {
            id: value.id.as_uuid(),
            ingredient_id: value.ingredient_id.map(|id| id.as_uuid()),
            prepared_meal_id: value.prepared_meal_id.map(|id| id.as_uuid()),
            product_id: value.product_id.map(|id| id.as_uuid()),
            name: value.name,
            quantity: value.quantity.map(Into::into),
            section: value.section,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingTripDto {
    pub id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-05")]
    pub opportunity_date: Date,
    pub state: TripStateDto,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub started_at: OffsetDateTime,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub finished_at: Option<OffsetDateTime>,
    pub rows: Vec<ShoppingTripRowDto>,
    pub revision: i64,
}

impl From<ShoppingTrip> for ShoppingTripDto {
    fn from(value: ShoppingTrip) -> Self {
        Self {
            id: value.id.as_uuid(),
            opportunity_date: value.opportunity_date,
            state: value.state.into(),
            started_at: value.started_at,
            finished_at: value.finished_at,
            rows: value.rows.into_iter().map(Into::into).collect(),
            revision: value.revision.get(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateShoppingListItemRequest {
    #[serde(default)]
    pub ingredient_id: Option<Uuid>,
    #[serde(default)]
    pub prepared_meal_id: Option<Uuid>,
    #[serde(default)]
    pub product_id: Option<Uuid>,
    #[schema(example = "Onion Salt")]
    pub name: String,
    #[serde(default)]
    pub quantity: Option<QuantityDto>,
    #[serde(default)]
    pub section: Option<ShoppingSection>,
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub opportunity_date: Option<Date>,
}

impl From<CreateShoppingListItemRequest> for NewShoppingListItem {
    fn from(value: CreateShoppingListItemRequest) -> Self {
        Self {
            ingredient_id: value.ingredient_id.map(Into::into),
            prepared_meal_id: value.prepared_meal_id.map(Into::into),
            product_id: value.product_id.map(Into::into),
            name: value.name,
            quantity: value.quantity.map(Into::into),
            section: value.section,
            opportunity_date: value.opportunity_date,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateShoppingListItemRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<QuantityDto>)]
    pub quantity: Patch<QuantityDto>,
    #[serde(default)]
    #[schema(value_type = Option<ShoppingSection>)]
    pub section: Patch<ShoppingSection>,
    #[serde(default)]
    #[schema(value_type = Option<String>, format = Date)]
    pub opportunity_date: Patch<Date>,
}

impl From<UpdateShoppingListItemRequest> for ShoppingListItemPatch {
    fn from(value: UpdateShoppingListItemRequest) -> Self {
        Self {
            name: value.name,
            quantity: value.quantity.map(Into::into),
            section: value.section,
            opportunity_date: value.opportunity_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShoppingListItemDto {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingredient_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_meal_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<QuantityDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<ShoppingSection>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date::option"
    )]
    #[schema(value_type = Option<String>, format = Date)]
    pub opportunity_date: Option<Date>,
    pub revision: i64,
}

impl From<ShoppingListItem> for ShoppingListItemDto {
    fn from(value: ShoppingListItem) -> Self {
        Self {
            id: value.id.as_uuid(),
            ingredient_id: value.ingredient_id.map(|id| id.as_uuid()),
            prepared_meal_id: value.prepared_meal_id.map(|id| id.as_uuid()),
            product_id: value.product_id.map(|id| id.as_uuid()),
            name: value.name,
            quantity: value.quantity.map(Into::into),
            section: value.section,
            opportunity_date: value.opportunity_date,
            revision: value.revision.get(),
        }
    }
}

impl From<ShoppingList> for ShoppingListDto {
    fn from(value: ShoppingList) -> Self {
        Self {
            opportunities: value.opportunities.into_iter().map(Into::into).collect(),
            focus: value.focus,
            requirements: value.requirements.into_iter().map(Into::into).collect(),
            manual: value.manual.into_iter().map(Into::into).collect(),
            unplanned: value.unplanned.into_iter().map(Into::into).collect(),
            trip: value.trip.map(Into::into),
            counts: value.counts.into_iter().map(Into::into).collect(),
            cadence_configured: value.cadence_configured,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseStateDto {
    Pending,
    Reconciled,
    Cancelled,
}

impl From<PurchaseState> for PurchaseStateDto {
    fn from(value: PurchaseState) -> Self {
        match value {
            PurchaseState::Pending => Self::Pending,
            PurchaseState::Reconciled => Self::Reconciled,
            PurchaseState::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PurchaseDto {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingredient_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_meal_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<QuantityDto>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date::option"
    )]
    #[schema(value_type = Option<String>, format = Date)]
    pub opportunity_date: Option<Date>,
    pub state: PurchaseStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_item_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub purchased_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub revision: i64,
}

impl From<Purchase> for PurchaseDto {
    fn from(value: Purchase) -> Self {
        Self {
            id: value.id.as_uuid(),
            ingredient_id: value.ingredient_id.map(|id| id.as_uuid()),
            prepared_meal_id: value.prepared_meal_id.map(|id| id.as_uuid()),
            product_id: value.product_id.map(|id| id.as_uuid()),
            name: value.name,
            quantity: value.quantity.map(Into::into),
            opportunity_date: value.opportunity_date,
            state: value.state.into(),
            stock_item_id: value.stock_item_id.map(|id| id.as_uuid()),
            purchased_at: value.purchased_at,
            note: value.note,
            revision: value.revision.get(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FinishShopResponse {
    pub stocked: usize,
    pub still_pending: usize,
}

impl From<FinishedShop> for FinishShopResponse {
    fn from(value: FinishedShop) -> Self {
        Self {
            stocked: value.stocked,
            still_pending: value.still_pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PurchasePage {
    pub items: Vec<PurchaseDto>,
    pub page: PageMeta,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePurchaseRequest {
    #[serde(default)]
    pub ingredient_id: Option<Uuid>,
    #[serde(default)]
    pub prepared_meal_id: Option<Uuid>,
    #[serde(default)]
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub quantity: Option<QuantityDto>,
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub opportunity_date: Option<Date>,
    #[serde(default)]
    pub note: Option<String>,
}

impl From<CreatePurchaseRequest> for NewPurchase {
    fn from(value: CreatePurchaseRequest) -> Self {
        Self {
            ingredient_id: value.ingredient_id.map(Into::into),
            prepared_meal_id: value.prepared_meal_id.map(Into::into),
            product_id: value.product_id.map(Into::into),
            name: value.name,
            quantity: value.quantity.map(Into::into),
            opportunity_date: value.opportunity_date,
            note: value.note,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdatePurchaseRequest {
    #[serde(default)]
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub quantity: Option<QuantityDto>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub note: mmp_core::domain::Patch<String>,
    #[serde(default)]
    pub cancelled: Option<bool>,
}

impl From<UpdatePurchaseRequest> for PurchasePatch {
    fn from(value: UpdatePurchaseRequest) -> Self {
        Self {
            product_id: value.product_id.map(Into::into),
            quantity: value.quantity.map(Into::into),
            note: value.note,
            cancelled: value.cancelled,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MoveOpportunityRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-10")]
    pub to: Date,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateOpportunityRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-02")]
    pub date: Date,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ShoppingListQuery {
    #[serde(default, with = "iso_date::option")]
    #[param(value_type = Option<String>, format = Date)]
    pub opportunity_date: Option<Date>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct OpportunityRangeQuery {
    #[serde(default, with = "iso_date::option")]
    #[param(value_type = Option<String>, format = Date)]
    pub from: Option<Date>,
    #[serde(default, with = "iso_date::option")]
    #[param(value_type = Option<String>, format = Date)]
    pub to: Option<Date>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PurchaseListQuery {
    pub state: Option<PurchaseStateDto>,
    #[serde(default, with = "iso_date::option")]
    #[param(value_type = Option<String>, format = Date)]
    pub opportunity_date: Option<Date>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl From<PurchaseStateDto> for PurchaseState {
    fn from(value: PurchaseStateDto) -> Self {
        match value {
            PurchaseStateDto::Pending => Self::Pending,
            PurchaseStateDto::Reconciled => Self::Reconciled,
            PurchaseStateDto::Cancelled => Self::Cancelled,
        }
    }
}

pub fn purchase_id(id: Uuid) -> mmp_core::domain::PurchaseId {
    mmp_core::domain::PurchaseId::from(id)
}

pub fn list_item_id(id: Uuid) -> mmp_core::domain::ShoppingListItemId {
    mmp_core::domain::ShoppingListItemId::from(id)
}

pub type RequirementSubject = DemandSubject;
