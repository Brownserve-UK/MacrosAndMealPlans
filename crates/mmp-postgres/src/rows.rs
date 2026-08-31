use std::collections::BTreeMap;
use std::str::FromStr;

use mmp_core::domain::{
    AccessScope, CatalogueOrigin, ConsumedAmount, ConsumptionRecord, ConsumptionRecordId,
    HouseholdMember, HouseholdMemberId, HouseholdSettings, Ingredient, IngredientId, MealItemRef,
    MealPlanComponentId, MealPlanEntryId, MealSlot, MealTimes, MemberAccessGrant,
    MissingStockInterpretation, NutritionFacts, NutritionGoals, NutritionQuality, NutritionTarget,
    NutritionTargetId, Product, ProductId, Provenance, Quantity, RecipeId, Revision, Role,
    SourceDate, SourceDateKind, StockEffect, StockEffectId, StockEffectSource, StockEffectState,
    StockEvent, StockEventId, StockEventKind, StockEventSource, StockItem, StockItemId, StockLevel,
    StorageLocation, TrackingMode, Unit, UsabilityDeadline, User, UserId,
};
use mmp_core::{CoreError, RepositoryError};
use rust_decimal::Decimal;
use sqlx::types::Json;
use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

type Extra = Json<BTreeMap<String, Decimal>>;

pub(crate) fn bad_value(column: &str, value: &str) -> CoreError {
    CoreError::Repository(RepositoryError::new(format!(
        "column `{column}` holds `{value}`, which this build does not understand"
    )))
}

#[derive(Debug, sqlx::FromRow)]
pub struct IngredientRow {
    pub id: Uuid,
    pub name: String,
    pub default_unit: String,
    pub origin: String,
    pub seed_key: Option<String>,
    pub source_provider: Option<String>,
    pub source_external_id: Option<String>,
    pub locally_modified: bool,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl TryFrom<IngredientRow> for Ingredient {
    type Error = CoreError;

    fn try_from(row: IngredientRow) -> Result<Self, Self::Error> {
        Ok(Ingredient {
            id: IngredientId::from(row.id),
            name: row.name,
            default_unit: Unit::from_str(&row.default_unit)
                .map_err(|_| bad_value("default_unit", &row.default_unit))?,
            provenance: Provenance {
                origin: CatalogueOrigin::from_str(&row.origin)
                    .map_err(|_| bad_value("origin", &row.origin))?,
                seed_key: row.seed_key,
                source_provider: row.source_provider,
                source_external_id: row.source_external_id,
                locally_modified: row.locally_modified,
            },
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct ProductRow {
    pub id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub retailer: Option<String>,
    pub shopping_section: Option<String>,
    pub package_quantity_amount: Option<Decimal>,
    pub package_quantity_unit: Option<String>,
    pub servings_per_pack: Option<i32>,
    pub mapped_ingredient_id: Option<Uuid>,
    pub nutrition_basis_amount: Option<Decimal>,
    pub nutrition_basis_unit: Option<String>,
    pub energy_kcal: Option<Decimal>,
    pub protein_g: Option<Decimal>,
    pub carbohydrate_g: Option<Decimal>,
    pub sugar_g: Option<Decimal>,
    pub fat_g: Option<Decimal>,
    pub saturated_fat_g: Option<Decimal>,
    pub fibre_g: Option<Decimal>,
    pub salt_g: Option<Decimal>,
    pub cholesterol_mg: Option<Decimal>,
    pub nutrition_extra: Extra,
    pub origin: String,
    pub seed_key: Option<String>,
    pub source_provider: Option<String>,
    pub source_external_id: Option<String>,
    pub locally_modified: bool,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl TryFrom<ProductRow> for Product {
    type Error = CoreError;

    fn try_from(row: ProductRow) -> Result<Self, Self::Error> {
        let package_quantity = match (row.package_quantity_amount, row.package_quantity_unit) {
            (Some(amount), Some(unit)) => Some(Quantity::new(
                amount,
                Unit::from_str(&unit).map_err(|_| bad_value("package_quantity_unit", &unit))?,
            )),
            _ => None,
        };

        Ok(Product {
            id: ProductId::from(row.id),
            name: row.name,
            brand: row.brand,
            barcode: row.barcode,
            retailer: row.retailer,
            shopping_section: row.shopping_section,
            package_quantity,
            servings_per_pack: row.servings_per_pack,
            mapped_ingredient_id: row.mapped_ingredient_id.map(IngredientId::from),
            nutrition: NutritionFacts {
                basis: parse_basis(row.nutrition_basis_amount, row.nutrition_basis_unit)?,
                energy_kcal: row.energy_kcal,
                protein_g: row.protein_g,
                carbohydrate_g: row.carbohydrate_g,
                sugar_g: row.sugar_g,
                fat_g: row.fat_g,
                saturated_fat_g: row.saturated_fat_g,
                fibre_g: row.fibre_g,
                salt_g: row.salt_g,
                cholesterol_mg: row.cholesterol_mg,
                extra: row.nutrition_extra.0,
            },
            provenance: Provenance {
                origin: CatalogueOrigin::from_str(&row.origin)
                    .map_err(|_| bad_value("origin", &row.origin))?,
                seed_key: row.seed_key,
                source_provider: row.source_provider,
                source_external_id: row.source_external_id,
                locally_modified: row.locally_modified,
            },
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

pub(crate) fn parse_basis(
    amount: Option<Decimal>,
    unit: Option<String>,
) -> Result<Option<Quantity>, CoreError> {
    match (amount, unit) {
        (Some(amount), Some(unit)) => {
            let unit =
                Unit::from_str(&unit).map_err(|_| bad_value("nutrition_basis_unit", &unit))?;
            Ok(Some(Quantity::new(amount, unit)))
        }
        _ => Ok(None),
    }
}

pub fn nutrition_bindings(facts: &NutritionFacts) -> NutritionBindings<'_> {
    NutritionBindings {
        basis_amount: facts.basis.map(|b| b.amount),
        basis_unit: facts.basis.map(|b| b.unit.code()),
        energy_kcal: facts.energy_kcal,
        protein_g: facts.protein_g,
        carbohydrate_g: facts.carbohydrate_g,
        sugar_g: facts.sugar_g,
        fat_g: facts.fat_g,
        saturated_fat_g: facts.saturated_fat_g,
        fibre_g: facts.fibre_g,
        salt_g: facts.salt_g,
        cholesterol_mg: facts.cholesterol_mg,
        extra: Json(&facts.extra),
    }
}

pub struct NutritionBindings<'a> {
    pub basis_amount: Option<Decimal>,
    pub basis_unit: Option<&'static str>,
    pub energy_kcal: Option<Decimal>,
    pub protein_g: Option<Decimal>,
    pub carbohydrate_g: Option<Decimal>,
    pub sugar_g: Option<Decimal>,
    pub fat_g: Option<Decimal>,
    pub saturated_fat_g: Option<Decimal>,
    pub fibre_g: Option<Decimal>,
    pub salt_g: Option<Decimal>,
    pub cholesterol_mg: Option<Decimal>,
    pub extra: Json<&'a BTreeMap<String, Decimal>>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct HouseholdMemberRow {
    pub id: Uuid,
    pub display_name: String,
    pub linked_user_id: Option<Uuid>,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl From<HouseholdMemberRow> for HouseholdMember {
    fn from(row: HouseholdMemberRow) -> Self {
        HouseholdMember {
            id: HouseholdMemberId::from(row.id),
            display_name: row.display_name,
            linked_user_id: row.linked_user_id.map(UserId::from),
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub auth_subject: Option<String>,
    pub roles: Vec<String>,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl TryFrom<UserRow> for User {
    type Error = CoreError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let mut roles = row
            .roles
            .iter()
            .map(|code| Role::from_str(code).map_err(|_| bad_value("role", code)))
            .collect::<Result<Vec<Role>, CoreError>>()?;
        roles.sort_unstable();
        roles.dedup();

        Ok(User {
            id: UserId::from(row.id),
            username: row.username,
            display_name: row.display_name,
            auth_subject: row.auth_subject,
            roles,
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct MemberAccessGrantRow {
    pub grantee_user_id: Uuid,
    pub subject_member_id: Uuid,
    pub scope: String,
    pub granted_at: OffsetDateTime,
    pub granted_by: Option<Uuid>,
}

impl TryFrom<MemberAccessGrantRow> for MemberAccessGrant {
    type Error = CoreError;

    fn try_from(row: MemberAccessGrantRow) -> Result<Self, Self::Error> {
        Ok(MemberAccessGrant {
            grantee_user_id: UserId::from(row.grantee_user_id),
            subject_member_id: HouseholdMemberId::from(row.subject_member_id),
            scope: AccessScope::from_str(&row.scope).map_err(|_| bad_value("scope", &row.scope))?,
            granted_at: row.granted_at,
            granted_by: row.granted_by.map(UserId::from),
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct NutritionTargetRow {
    pub id: Uuid,
    pub member_id: Uuid,
    pub effective_from: Date,
    pub energy_kcal: Option<Decimal>,
    pub protein_g: Option<Decimal>,
    pub carbohydrate_g: Option<Decimal>,
    pub sugar_g: Option<Decimal>,
    pub fat_g: Option<Decimal>,
    pub saturated_fat_g: Option<Decimal>,
    pub fibre_g: Option<Decimal>,
    pub salt_g: Option<Decimal>,
    pub cholesterol_mg: Option<Decimal>,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<NutritionTargetRow> for NutritionTarget {
    fn from(row: NutritionTargetRow) -> Self {
        NutritionTarget {
            id: NutritionTargetId::from(row.id),
            member_id: HouseholdMemberId::from(row.member_id),
            effective_from: row.effective_from,
            goals: NutritionGoals {
                energy_kcal: row.energy_kcal,
                protein_g: row.protein_g,
                carbohydrate_g: row.carbohydrate_g,
                sugar_g: row.sugar_g,
                fat_g: row.fat_g,
                saturated_fat_g: row.saturated_fat_g,
                fibre_g: row.fibre_g,
                salt_g: row.salt_g,
                cholesterol_mg: row.cholesterol_mg,
            },
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct HouseholdSettingsRow {
    pub breakfast_time: Time,
    pub lunch_time: Time,
    pub dinner_time: Time,
    pub missing_stock_interpretation: String,
    pub default_all_members_participate: bool,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl TryFrom<HouseholdSettingsRow> for HouseholdSettings {
    type Error = CoreError;

    fn try_from(row: HouseholdSettingsRow) -> Result<Self, Self::Error> {
        Ok(HouseholdSettings {
            meal_times: MealTimes {
                breakfast: row.breakfast_time,
                lunch: row.lunch_time,
                dinner: row.dinner_time,
            },
            missing_stock_interpretation: MissingStockInterpretation::from_str(
                &row.missing_stock_interpretation,
            )
            .map_err(|_| {
                bad_value(
                    "missing_stock_interpretation",
                    &row.missing_stock_interpretation,
                )
            })?,
            default_all_members_participate: row.default_all_members_participate,
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct StockItemRow {
    pub id: Uuid,
    pub product_id: Uuid,
    pub tracking_mode: String,
    pub quantity_value: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub estimated_low: Option<Decimal>,
    pub estimated_high: Option<Decimal>,
    pub storage_location: String,
    pub source_date: Option<Date>,
    pub source_date_kind: Option<String>,
    pub usability_deadline: Option<Date>,
    pub usability_deadline_basis: Option<String>,
    pub note: Option<String>,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl TryFrom<StockItemRow> for StockItem {
    type Error = CoreError;

    fn try_from(row: StockItemRow) -> Result<Self, Self::Error> {
        let mode = TrackingMode::from_str(&row.tracking_mode)
            .map_err(|_| bad_value("tracking_mode", &row.tracking_mode))?;
        let unit = match &row.quantity_unit {
            Some(code) => Some(Unit::from_str(code).map_err(|_| bad_value("quantity_unit", code))?),
            None => None,
        };
        let level = match mode {
            TrackingMode::Exact => StockLevel::Exact {
                quantity: Quantity::new(
                    row.quantity_value
                        .ok_or_else(|| bad_value("quantity_value", "null"))?,
                    unit.ok_or_else(|| bad_value("quantity_unit", "null"))?,
                ),
            },
            TrackingMode::Estimated => StockLevel::Estimated {
                low: row
                    .estimated_low
                    .ok_or_else(|| bad_value("estimated_low", "null"))?,
                high: row
                    .estimated_high
                    .ok_or_else(|| bad_value("estimated_high", "null"))?,
                unit: unit.ok_or_else(|| bad_value("quantity_unit", "null"))?,
            },
            TrackingMode::NotTracked => StockLevel::NotTracked,
        };

        let source_date = match (row.source_date, row.source_date_kind) {
            (Some(date), Some(kind)) => Some(SourceDate {
                date,
                kind: SourceDateKind::from_str(&kind)
                    .map_err(|_| bad_value("source_date_kind", &kind))?,
            }),
            _ => None,
        };
        let usability_deadline = row.usability_deadline.map(|date| UsabilityDeadline {
            date,
            basis: row.usability_deadline_basis,
        });

        Ok(StockItem {
            id: StockItemId::from(row.id),
            product_id: ProductId::from(row.product_id),
            level,
            storage_location: StorageLocation::from_str(&row.storage_location)
                .map_err(|_| bad_value("storage_location", &row.storage_location))?,
            source_date,
            usability_deadline,
            note: row.note,
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct StockEventRow {
    pub id: Uuid,
    pub stock_item_id: Uuid,
    pub event_kind: String,
    pub quantity_delta: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub subject_member_id: Option<Uuid>,
    pub source_kind: Option<String>,
    pub source_id: Option<Uuid>,
    pub source_label: Option<String>,
    pub reverses_event_id: Option<Uuid>,
    pub note: Option<String>,
    pub occurred_at: OffsetDateTime,
}

impl TryFrom<StockEventRow> for StockEvent {
    type Error = CoreError;

    fn try_from(row: StockEventRow) -> Result<Self, Self::Error> {
        let quantity_delta = match (row.quantity_delta, row.quantity_unit) {
            (Some(amount), Some(unit)) => Some(Quantity::new(
                amount,
                Unit::from_str(&unit).map_err(|_| bad_value("quantity_unit", &unit))?,
            )),
            _ => None,
        };
        let source = match (row.source_kind, row.source_id, row.source_label) {
            (Some(kind), Some(id), label) => Some(StockEventSource {
                kind: StockEffectSource::from_str(&kind)
                    .map_err(|_| bad_value("source_kind", &kind))?,
                id,
                label: label.unwrap_or_default(),
            }),
            _ => None,
        };
        Ok(StockEvent {
            id: StockEventId::from(row.id),
            stock_item_id: StockItemId::from(row.stock_item_id),
            kind: StockEventKind::from_str(&row.event_kind)
                .map_err(|_| bad_value("event_kind", &row.event_kind))?,
            quantity_delta,
            actor_user_id: row.actor_user_id.map(UserId::from),
            subject_member_id: row.subject_member_id.map(HouseholdMemberId::from),
            source,
            reverses_event_id: row.reverses_event_id.map(StockEventId::from),
            note: row.note,
            occurred_at: row.occurred_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct StockEffectRow {
    pub id: Uuid,
    pub source_kind: String,
    pub source_id: Uuid,
    pub stock_item_id: Uuid,
    pub product_id: Uuid,
    pub state: String,
    pub applied_mode: String,
    pub applied_unit: String,
    pub exact_delta: Option<Decimal>,
    pub low_delta: Option<Decimal>,
    pub high_delta: Option<Decimal>,
    pub requested_value: Decimal,
    pub apply_event_id: Uuid,
    pub applied_at: OffsetDateTime,
    pub released_at: Option<OffsetDateTime>,
    pub note: Option<String>,
}

impl TryFrom<StockEffectRow> for StockEffect {
    type Error = CoreError;

    fn try_from(row: StockEffectRow) -> Result<Self, Self::Error> {
        Ok(StockEffect {
            id: StockEffectId::from(row.id),
            source_kind: StockEffectSource::from_str(&row.source_kind)
                .map_err(|_| bad_value("source_kind", &row.source_kind))?,
            source_id: row.source_id,
            stock_item_id: StockItemId::from(row.stock_item_id),
            product_id: ProductId::from(row.product_id),
            state: StockEffectState::from_str(&row.state)
                .map_err(|_| bad_value("state", &row.state))?,
            applied_mode: TrackingMode::from_str(&row.applied_mode)
                .map_err(|_| bad_value("applied_mode", &row.applied_mode))?,
            applied_unit: Unit::from_str(&row.applied_unit)
                .map_err(|_| bad_value("applied_unit", &row.applied_unit))?,
            exact_delta: row.exact_delta,
            low_delta: row.low_delta,
            high_delta: row.high_delta,
            requested_value: row.requested_value,
            apply_event_id: StockEventId::from(row.apply_event_id),
            applied_at: row.applied_at,
            released_at: row.released_at,
            note: row.note,
        })
    }
}

pub fn amount_bindings(amount: &ConsumedAmount) -> (&'static str, Decimal, Option<&'static str>) {
    match amount {
        ConsumedAmount::Measure(quantity) => (
            amount.kind_code(),
            quantity.amount,
            Some(quantity.unit.code()),
        ),
        ConsumedAmount::Servings(value) | ConsumedAmount::Packs(value) => {
            (amount.kind_code(), *value, None)
        }
    }
}

pub fn item_bindings(item: &MealItemRef) -> (&'static str, Option<Uuid>, Option<Uuid>) {
    (
        item.kind_code(),
        item.product_id().map(|id| id.as_uuid()),
        item.recipe_id().map(|id| id.as_uuid()),
    )
}

pub(crate) fn parse_amount(
    kind: &str,
    value: Decimal,
    unit: Option<String>,
) -> Result<ConsumedAmount, CoreError> {
    match kind {
        "measure" => {
            let unit = unit.ok_or_else(|| bad_value("amount_unit", "null"))?;
            let unit = Unit::from_str(&unit).map_err(|_| bad_value("amount_unit", &unit))?;
            Ok(ConsumedAmount::Measure(Quantity::new(value, unit)))
        }
        "servings" => Ok(ConsumedAmount::Servings(value)),
        "packs" => Ok(ConsumedAmount::Packs(value)),
        _ => Err(bad_value("amount_kind", kind)),
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct ConsumptionRecordRow {
    pub id: Uuid,
    pub member_id: Uuid,
    pub item_kind: String,
    pub product_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub recorded_by: Option<Uuid>,
    pub meal_plan_entry_id: Option<Uuid>,
    pub meal_plan_component_id: Option<Uuid>,
    pub slot: String,
    pub amount_kind: String,
    pub amount_value: Decimal,
    pub amount_unit: Option<String>,
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub nutrition_basis_amount: Option<Decimal>,
    pub nutrition_basis_unit: Option<String>,
    pub energy_kcal: Option<Decimal>,
    pub protein_g: Option<Decimal>,
    pub carbohydrate_g: Option<Decimal>,
    pub sugar_g: Option<Decimal>,
    pub fat_g: Option<Decimal>,
    pub saturated_fat_g: Option<Decimal>,
    pub fibre_g: Option<Decimal>,
    pub salt_g: Option<Decimal>,
    pub cholesterol_mg: Option<Decimal>,
    pub nutrition_extra: Extra,
    pub nutrition_quality: String,
    pub revision: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl TryFrom<ConsumptionRecordRow> for ConsumptionRecord {
    type Error = CoreError;

    fn try_from(row: ConsumptionRecordRow) -> Result<Self, Self::Error> {
        let amount = parse_amount(&row.amount_kind, row.amount_value, row.amount_unit)?;

        Ok(ConsumptionRecord {
            id: ConsumptionRecordId::from(row.id),
            member_id: HouseholdMemberId::from(row.member_id),
            item: MealItemRef::from_parts(
                &row.item_kind,
                row.product_id.map(ProductId::from),
                row.recipe_id.map(RecipeId::from),
            )
            .map_err(|_| bad_value("item_kind", &row.item_kind))?,
            recorded_by: row.recorded_by.map(UserId::from),
            meal_plan_entry_id: row.meal_plan_entry_id.map(MealPlanEntryId::from),
            meal_plan_component_id: row.meal_plan_component_id.map(MealPlanComponentId::from),
            slot: MealSlot::from_str(&row.slot).map_err(|_| bad_value("slot", &row.slot))?,
            amount,
            consumed_on: row.consumed_on,
            consumed_at: row.consumed_at,
            nutrition: NutritionFacts {
                basis: parse_basis(row.nutrition_basis_amount, row.nutrition_basis_unit)?,
                energy_kcal: row.energy_kcal,
                protein_g: row.protein_g,
                carbohydrate_g: row.carbohydrate_g,
                sugar_g: row.sugar_g,
                fat_g: row.fat_g,
                saturated_fat_g: row.saturated_fat_g,
                fibre_g: row.fibre_g,
                salt_g: row.salt_g,
                cholesterol_mg: row.cholesterol_mg,
                extra: row.nutrition_extra.0,
            },
            quality: NutritionQuality::from_str(&row.nutrition_quality)
                .map_err(|_| bad_value("nutrition_quality", &row.nutrition_quality))?,
            revision: Revision::new(row.revision),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
