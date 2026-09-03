use mmp_core::domain::{
    Assignment, Certainty, DemandSubject, NewPurchase, NewShoppingCadence, OpportunityState,
    Purchase, PurchasePatch, PurchaseState, ShoppingCadence, ShoppingOpportunity,
    ShoppingRequirement, ShoppingSection, SuggestionReason, week_day_from_number, week_day_number,
};
use mmp_core::services::ShoppingList;
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
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionReasonDto {
    UnknownAvailability,
    AssumptionOnly,
}

impl From<SuggestionReason> for SuggestionReasonDto {
    fn from(value: SuggestionReason) -> Self {
        match value {
            SuggestionReason::UnknownAvailability => Self::UnknownAvailability,
            SuggestionReason::AssumptionOnly => Self::AssumptionOnly,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase: Option<PurchaseDto>,
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
            purchase: value.purchase.map(Into::into),
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
    pub cadence_configured: bool,
}

impl From<ShoppingList> for ShoppingListDto {
    fn from(value: ShoppingList) -> Self {
        Self {
            opportunities: value.opportunities.into_iter().map(Into::into).collect(),
            focus: value.focus,
            requirements: value.requirements.into_iter().map(Into::into).collect(),
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
    pub product_id: Option<Uuid>,
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
            product_id: value.product_id.map(|id| id.as_uuid()),
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
pub struct PurchasePage {
    pub items: Vec<PurchaseDto>,
    pub page: PageMeta,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePurchaseRequest {
    #[serde(default)]
    pub ingredient_id: Option<Uuid>,
    #[serde(default)]
    pub product_id: Option<Uuid>,
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
            product_id: value.product_id.map(Into::into),
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

pub type RequirementSubject = DemandSubject;
