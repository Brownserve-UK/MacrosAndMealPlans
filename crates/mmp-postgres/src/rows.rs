use std::collections::BTreeMap;
use std::str::FromStr;

use mmp_core::domain::{
    AccessScope, CatalogueOrigin, ConsumedAmount, ConsumptionRecord, ConsumptionRecordId,
    HouseholdMember, HouseholdMemberId, Ingredient, IngredientId, MemberAccessGrant,
    NutritionFacts, NutritionQuality, Product, ProductId, Provenance, Quantity, Revision, Role,
    Unit, User, UserId,
};
use mmp_core::{CoreError, RepositoryError};
use rust_decimal::Decimal;
use sqlx::types::Json;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

type Extra = Json<BTreeMap<String, Decimal>>;

fn bad_value(column: &str, value: &str) -> CoreError {
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

fn parse_basis(
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

fn parse_amount(
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
    pub product_id: Uuid,
    pub recorded_by: Option<Uuid>,
    pub amount_kind: String,
    pub amount_value: Decimal,
    pub amount_unit: Option<String>,
    pub consumed_on: Date,
    pub consumed_at: OffsetDateTime,
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
            product_id: ProductId::from(row.product_id),
            recorded_by: row.recorded_by.map(UserId::from),
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
