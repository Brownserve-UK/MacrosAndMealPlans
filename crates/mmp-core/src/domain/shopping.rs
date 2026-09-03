use std::fmt;
use std::str::FromStr;

use time::{Date, OffsetDateTime, Time, Weekday};

use super::{
    DemandClaim, DemandGap, DemandSubject, IngredientId, ProductId, PurchaseId, Quantity, Revision,
    ShoppingOpportunityId, StockItemId, UserId,
};
use crate::error::{Result, ValidationErrors};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ShoppingSection {
    FreshProduce,
    MeatFish,
    Dairy,
    Bakery,
    Frozen,
    Ambient,
    Drinks,
    Household,
    Other,
}

impl ShoppingSection {
    pub const ALL: [ShoppingSection; 9] = [
        ShoppingSection::FreshProduce,
        ShoppingSection::MeatFish,
        ShoppingSection::Dairy,
        ShoppingSection::Bakery,
        ShoppingSection::Frozen,
        ShoppingSection::Ambient,
        ShoppingSection::Drinks,
        ShoppingSection::Household,
        ShoppingSection::Other,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            ShoppingSection::FreshProduce => "fresh_produce",
            ShoppingSection::MeatFish => "meat_fish",
            ShoppingSection::Dairy => "dairy",
            ShoppingSection::Bakery => "bakery",
            ShoppingSection::Frozen => "frozen",
            ShoppingSection::Ambient => "ambient",
            ShoppingSection::Drinks => "drinks",
            ShoppingSection::Household => "household",
            ShoppingSection::Other => "other",
        }
    }

    pub fn order(&self) -> usize {
        ShoppingSection::ALL
            .iter()
            .position(|section| section == self)
            .unwrap_or(usize::MAX)
    }
}

impl fmt::Display for ShoppingSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown shopping section: {0}")]
pub struct UnknownShoppingSection(pub String);

impl FromStr for ShoppingSection {
    type Err = UnknownShoppingSection;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        ShoppingSection::ALL
            .into_iter()
            .find(|section| section.code() == s)
            .ok_or_else(|| UnknownShoppingSection(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShoppingCadence {
    pub interval_weeks: u8,
    pub days: Vec<Weekday>,
    pub anchor: Date,
    pub usual_time: Option<Time>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewShoppingCadence {
    pub interval_weeks: u8,
    pub days: Vec<Weekday>,
    pub anchor: Date,
    pub usual_time: Option<Time>,
}

impl NewShoppingCadence {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if self.interval_weeks == 0 || self.interval_weeks > 8 {
            errors.push("interval_weeks", "Choose between 1 and 8 weeks.");
        }
        if self.days.is_empty() {
            errors.push("days", "Choose at least one day.");
        }
        let mut seen: Vec<u8> = self.days.iter().map(week_day_number).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        if seen.len() != before {
            errors.push("days", "Each day can only be chosen once.");
        }
        errors.into_result()
    }
}

pub fn week_day_number(day: &Weekday) -> u8 {
    day.number_from_monday()
}

pub fn week_day_from_number(number: u8) -> Option<Weekday> {
    Some(match number {
        1 => Weekday::Monday,
        2 => Weekday::Tuesday,
        3 => Weekday::Wednesday,
        4 => Weekday::Thursday,
        5 => Weekday::Friday,
        6 => Weekday::Saturday,
        7 => Weekday::Sunday,
        _ => return None,
    })
}

impl ShoppingCadence {
    pub fn occurrences(&self, from: Date, to: Date) -> Vec<Date> {
        if self.days.is_empty() || self.interval_weeks == 0 || from > to {
            return Vec::new();
        }
        let anchor_monday = monday_of(self.anchor);
        let mut dates = Vec::new();
        let mut day = from;
        while day <= to {
            let weeks = (monday_of(day) - anchor_monday).whole_days() / 7;
            if weeks.rem_euclid(i64::from(self.interval_weeks)) == 0
                && self.days.contains(&day.weekday())
            {
                dates.push(day);
            }
            let Some(next) = day.next_day() else { break };
            day = next;
        }
        dates
    }
}

fn monday_of(date: Date) -> Date {
    let back = i64::from(date.weekday().number_days_from_monday());
    date - time::Duration::days(back)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityState {
    Normal,
    Moved,
    OneOff,
}

impl OpportunityState {
    pub const ALL: [OpportunityState; 3] = [
        OpportunityState::Normal,
        OpportunityState::Moved,
        OpportunityState::OneOff,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            OpportunityState::Normal => "normal",
            OpportunityState::Moved => "moved",
            OpportunityState::OneOff => "one_off",
        }
    }
}

impl fmt::Display for OpportunityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown opportunity state: {0}")]
pub struct UnknownOpportunityState(pub String);

impl FromStr for OpportunityState {
    type Err = UnknownOpportunityState;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        OpportunityState::ALL
            .into_iter()
            .find(|state| state.code() == s)
            .ok_or_else(|| UnknownOpportunityState(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShoppingOpportunity {
    pub id: Option<ShoppingOpportunityId>,
    pub date: Date,
    pub state: OpportunityState,
    pub generated_for: Option<Date>,
    pub usual_time: Option<Time>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityException {
    pub id: ShoppingOpportunityId,
    pub generated_for: Option<Date>,
    pub effective_date: Option<Date>,
    pub usual_time: Option<Time>,
    pub state: ExceptionState,
    pub note: Option<String>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionState {
    Moved,
    Skipped,
    OneOff,
}

impl ExceptionState {
    pub const ALL: [ExceptionState; 3] = [
        ExceptionState::Moved,
        ExceptionState::Skipped,
        ExceptionState::OneOff,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            ExceptionState::Moved => "moved",
            ExceptionState::Skipped => "skipped",
            ExceptionState::OneOff => "one_off",
        }
    }
}

impl fmt::Display for ExceptionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown exception state: {0}")]
pub struct UnknownExceptionState(pub String);

impl FromStr for ExceptionState {
    type Err = UnknownExceptionState;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        ExceptionState::ALL
            .into_iter()
            .find(|state| state.code() == s)
            .ok_or_else(|| UnknownExceptionState(s.to_owned()))
    }
}

pub fn expand_opportunities(
    cadence: Option<&ShoppingCadence>,
    exceptions: &[OpportunityException],
    from: Date,
    to: Date,
) -> Vec<ShoppingOpportunity> {
    let generated = cadence
        .map(|cadence| cadence.occurrences(from, to))
        .unwrap_or_default();

    let mut out: Vec<ShoppingOpportunity> = Vec::new();

    for date in generated {
        match exceptions
            .iter()
            .find(|exception| exception.generated_for == Some(date))
        {
            None => out.push(ShoppingOpportunity {
                id: None,
                date,
                state: OpportunityState::Normal,
                generated_for: None,
                usual_time: cadence.and_then(|cadence| cadence.usual_time),
                note: None,
            }),
            Some(exception) => match exception.state {
                ExceptionState::Skipped => {}
                _ => {
                    if let Some(effective) = exception.effective_date {
                        out.push(ShoppingOpportunity {
                            id: Some(exception.id),
                            date: effective,
                            state: OpportunityState::Moved,
                            generated_for: exception.generated_for,
                            usual_time: exception.usual_time,
                            note: exception.note.clone(),
                        });
                    }
                }
            },
        }
    }

    for exception in exceptions
        .iter()
        .filter(|exception| exception.state == ExceptionState::OneOff)
    {
        if let Some(effective) = exception.effective_date
            && effective >= from
            && effective <= to
        {
            out.push(ShoppingOpportunity {
                id: Some(exception.id),
                date: effective,
                state: OpportunityState::OneOff,
                generated_for: None,
                usual_time: exception.usual_time,
                note: exception.note.clone(),
            });
        }
    }

    out.sort_by_key(|opportunity| opportunity.date);
    out.dedup_by_key(|opportunity| opportunity.date);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Certainty {
    Definite,
    Suggested { reason: SuggestionReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionReason {
    UnknownAvailability,
    AssumptionOnly,
}

impl SuggestionReason {
    pub const fn code(&self) -> &'static str {
        match self {
            SuggestionReason::UnknownAvailability => "unknown_availability",
            SuggestionReason::AssumptionOnly => "assumption_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Assignment {
    Opportunity { date: Date },
    NeedsEarlierOpportunity,
    Unassigned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShoppingRequirement {
    pub subject: DemandSubject,
    pub name: String,
    pub quantity: Option<Quantity>,
    pub required_by: Option<Date>,
    pub use_by_at_least: Option<Date>,
    pub section: ShoppingSection,
    pub certainty: Certainty,
    pub assignment: Assignment,
    pub claims: Vec<DemandClaim>,
    pub gaps: Vec<DemandGap>,
    pub purchase: Option<Purchase>,
}

pub fn assign(required_by: Option<Date>, opportunities: &[ShoppingOpportunity]) -> Assignment {
    if opportunities.is_empty() {
        return Assignment::Unassigned;
    }
    let Some(required_by) = required_by else {
        return Assignment::Opportunity {
            date: opportunities[0].date,
        };
    };
    match opportunities
        .iter()
        .filter(|opportunity| opportunity.date <= required_by)
        .map(|opportunity| opportunity.date)
        .max()
    {
        Some(date) => Assignment::Opportunity { date },
        None => Assignment::NeedsEarlierOpportunity,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseState {
    Pending,
    Reconciled,
    Cancelled,
}

impl PurchaseState {
    pub const ALL: [PurchaseState; 3] = [
        PurchaseState::Pending,
        PurchaseState::Reconciled,
        PurchaseState::Cancelled,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            PurchaseState::Pending => "pending",
            PurchaseState::Reconciled => "reconciled",
            PurchaseState::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for PurchaseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown purchase state: {0}")]
pub struct UnknownPurchaseState(pub String);

impl FromStr for PurchaseState {
    type Err = UnknownPurchaseState;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        PurchaseState::ALL
            .into_iter()
            .find(|state| state.code() == s)
            .ok_or_else(|| UnknownPurchaseState(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    pub id: PurchaseId,
    pub ingredient_id: Option<IngredientId>,
    pub product_id: Option<ProductId>,
    pub quantity: Option<Quantity>,
    pub opportunity_date: Option<Date>,
    pub state: PurchaseState,
    pub stock_item_id: Option<StockItemId>,
    pub purchased_at: OffsetDateTime,
    pub actor_user_id: UserId,
    pub note: Option<String>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Purchase {
    pub fn is_complete(&self) -> bool {
        self.product_id.is_some() && self.quantity.is_some()
    }

    pub fn matches(&self, subject: &DemandSubject, pool: &[ProductId]) -> bool {
        match subject {
            DemandSubject::Ingredient { ingredient_id } => {
                self.ingredient_id == Some(*ingredient_id)
                    || self
                        .product_id
                        .is_some_and(|product_id| pool.contains(&product_id))
            }
            DemandSubject::Product { product_id } => self.product_id == Some(*product_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPurchase {
    pub ingredient_id: Option<IngredientId>,
    pub product_id: Option<ProductId>,
    pub quantity: Option<Quantity>,
    pub opportunity_date: Option<Date>,
    pub note: Option<String>,
}

impl NewPurchase {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if self.ingredient_id.is_none() && self.product_id.is_none() {
            errors.push("product_id", "Say what was bought.");
        }
        if let Some(quantity) = self.quantity
            && quantity.amount <= rust_decimal::Decimal::ZERO
        {
            errors.push("quantity", "Enter an amount above zero.");
        }
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PurchasePatch {
    pub product_id: Option<ProductId>,
    pub quantity: Option<Quantity>,
    pub note: super::Patch<String>,
    pub cancelled: Option<bool>,
}

impl PurchasePatch {
    pub fn is_empty(&self) -> bool {
        self.product_id.is_none()
            && self.quantity.is_none()
            && self.note.is_unchanged()
            && self.cancelled.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(quantity) = self.quantity
            && quantity.amount <= rust_decimal::Decimal::ZERO
        {
            errors.push("quantity", "Enter an amount above zero.");
        }
        errors.into_result()
    }
}

#[cfg(test)]
#[path = "shopping_tests.rs"]
mod tests;
