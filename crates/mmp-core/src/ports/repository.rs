use std::collections::HashMap;

use async_trait::async_trait;
use time::Date;

use super::{PageRequest, Paginated};
use crate::domain::{
    AccessScope, CatalogueOrigin, ConsumptionRecord, ConsumptionRecordId, DeductionTarget,
    HouseholdMember, HouseholdMemberId, HouseholdSettings, Ingredient, IngredientId,
    MealParticipant, MealPlanComponentId, MealPlanComponentSnapshot, MealPlanEntry,
    MealPlanEntryId, MemberAccessGrant, NewStockEvent, NutritionTarget, NutritionTargetId, Product,
    ProductId, Quantity, Recipe, RecipeId, RecipePhoto, RecipeSummary, Revision, Role, StockEffect,
    StockEffectSource, StockEvent, StockItem, StockItemId, StockOutcome, User, UserId,
};
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    Updated,
    RevisionMismatch { actual: Revision },
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IngredientSort {
    #[default]
    Name,
    Created,
    ProductCount,
}

#[derive(Debug, Clone, Default)]
pub struct IngredientQuery {
    pub search: Option<String>,
    pub origin: Option<CatalogueOrigin>,
    pub needs_products: Option<bool>,
    pub include_archived: bool,
    pub page: PageRequest,
    pub sort_by: IngredientSort,
    pub sort: SortDirection,
}

#[derive(Debug, Clone, Default)]
pub struct ProductQuery {
    pub search: Option<String>,
    pub origin: Option<CatalogueOrigin>,
    pub barcode: Option<String>,
    pub retailer: Option<String>,
    pub mapped_ingredient_id: Option<IngredientId>,
    pub unmapped: Option<bool>,
    pub include_archived: bool,
    pub page: PageRequest,
    pub sort: SortDirection,
}

#[derive(Debug, Clone, Default)]
pub struct MemberQuery {
    pub search: Option<String>,
    pub with_account: Option<bool>,
    pub include_archived: bool,
    pub page: PageRequest,
    pub sort: SortDirection,
}

#[derive(Debug, Clone, Default)]
pub struct UserQuery {
    pub search: Option<String>,
    pub role: Option<Role>,
    pub include_archived: bool,
    pub page: PageRequest,
    pub sort: SortDirection,
}

#[derive(Debug, Clone, Default)]
pub struct ConsumptionQuery {
    pub member_id: Option<HouseholdMemberId>,
    pub from: Option<Date>,
    pub to: Option<Date>,
    pub page: PageRequest,
    pub sort: SortDirection,
}

#[derive(Debug, Clone)]
pub struct MealPlanQuery {
    pub member_id: HouseholdMemberId,
    pub from: Date,
    pub to: Date,
    pub include_participating: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StockQuery {
    pub product_id: Option<ProductId>,
    pub include_archived: bool,
    pub page: PageRequest,
    pub sort: SortDirection,
}

#[derive(Debug, Clone)]
pub struct RecipeQuery {
    pub owner_id: UserId,
    pub search: Option<String>,
    pub include_archived: bool,
    pub page: PageRequest,
    pub sort: SortDirection,
}

#[async_trait]
pub trait HouseholdMemberRepository: Send + Sync + 'static {
    async fn get(&self, id: HouseholdMemberId) -> Result<Option<HouseholdMember>>;

    async fn find_by_display_name(&self, name: &str) -> Result<Option<HouseholdMember>>;

    async fn find_by_linked_user(&self, user_id: UserId) -> Result<Option<HouseholdMember>>;

    async fn list(&self, query: &MemberQuery) -> Result<Paginated<HouseholdMember>>;

    async fn insert(&self, member: &HouseholdMember) -> Result<()>;

    async fn update(&self, member: &HouseholdMember, expected: Revision) -> Result<UpdateOutcome>;
}

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn get(&self, id: UserId) -> Result<Option<User>>;

    async fn find_by_username(&self, username: &str) -> Result<Option<User>>;

    async fn list(&self, query: &UserQuery) -> Result<Paginated<User>>;

    async fn count_with_role(&self, role: Role, include_archived: bool) -> Result<i64>;

    async fn insert(&self, user: &User) -> Result<()>;

    async fn update(&self, user: &User, expected: Revision) -> Result<UpdateOutcome>;
}

#[async_trait]
pub trait AccessGrantRepository: Send + Sync + 'static {
    async fn list_for_member(&self, member_id: HouseholdMemberId)
    -> Result<Vec<MemberAccessGrant>>;

    async fn exists(
        &self,
        grantee_user_id: UserId,
        subject_member_id: HouseholdMemberId,
        scope: AccessScope,
    ) -> Result<bool>;

    async fn upsert(&self, grant: &MemberAccessGrant) -> Result<()>;

    async fn revoke(
        &self,
        grantee_user_id: UserId,
        subject_member_id: HouseholdMemberId,
        scope: AccessScope,
    ) -> Result<bool>;
}

#[async_trait]
pub trait IngredientRepository: Send + Sync + 'static {
    async fn get(&self, id: IngredientId) -> Result<Option<Ingredient>>;

    async fn get_many(&self, ids: &[IngredientId]) -> Result<Vec<Ingredient>> {
        let mut ingredients = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(ingredient) = self.get(*id).await? {
                ingredients.push(ingredient);
            }
        }
        Ok(ingredients)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Ingredient>>;

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<Ingredient>>;

    async fn list(&self, query: &IngredientQuery) -> Result<Paginated<Ingredient>>;

    async fn insert(&self, ingredient: &Ingredient) -> Result<()>;

    async fn update(&self, ingredient: &Ingredient, expected: Revision) -> Result<UpdateOutcome>;
}

#[async_trait]
pub trait ProductRepository: Send + Sync + 'static {
    async fn get(&self, id: ProductId) -> Result<Option<Product>>;

    async fn get_many(&self, ids: &[ProductId]) -> Result<Vec<Product>> {
        let mut products = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(product) = self.get(*id).await? {
                products.push(product);
            }
        }
        Ok(products)
    }

    async fn find_by_barcode(&self, barcode: &str) -> Result<Option<Product>>;

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<Product>>;

    async fn list(&self, query: &ProductQuery) -> Result<Paginated<Product>>;

    async fn count_by_ingredient(
        &self,
        ingredient_ids: &[IngredientId],
    ) -> Result<HashMap<IngredientId, i64>>;

    async fn list_by_ingredient(
        &self,
        ingredient_ids: &[IngredientId],
    ) -> Result<HashMap<IngredientId, Vec<Product>>>;

    async fn insert(&self, product: &Product) -> Result<()>;

    async fn update(&self, product: &Product, expected: Revision) -> Result<UpdateOutcome>;
}

#[derive(Debug, Clone)]
pub struct StockDeduction {
    pub source_kind: StockEffectSource,
    pub source_id: uuid::Uuid,
    pub source_detail_id: Option<uuid::Uuid>,
    pub target: DeductionTarget,
    pub want: Quantity,
    pub actor_user_id: Option<UserId>,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub source_label: String,
}

#[derive(Debug, Clone)]
pub struct StockRelease {
    pub source_kind: StockEffectSource,
    pub source_id: uuid::Uuid,
    pub actor_user_id: Option<UserId>,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub source_label: String,
}

#[derive(Debug, Clone, Default)]
pub struct StockWrite {
    pub deductions: Vec<StockDeduction>,
    pub releases: Vec<StockRelease>,
}

impl StockWrite {
    pub fn is_empty(&self) -> bool {
        self.deductions.is_empty() && self.releases.is_empty()
    }
}

#[async_trait]
pub trait ConsumptionRecordRepository: Send + Sync + 'static {
    async fn get(&self, id: ConsumptionRecordId) -> Result<Option<ConsumptionRecord>>;

    async fn list(&self, query: &ConsumptionQuery) -> Result<Paginated<ConsumptionRecord>>;

    async fn list_period(
        &self,
        member_id: HouseholdMemberId,
        from: Date,
        to: Date,
    ) -> Result<Vec<ConsumptionRecord>>;

    async fn list_for_meal_plan_entry(
        &self,
        entry_id: MealPlanEntryId,
    ) -> Result<Vec<ConsumptionRecord>>;

    async fn insert(
        &self,
        record: &ConsumptionRecord,
        stock: &StockWrite,
    ) -> Result<Vec<StockOutcome>>;

    async fn update(
        &self,
        record: &ConsumptionRecord,
        expected: Revision,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)>;

    async fn delete(
        &self,
        id: ConsumptionRecordId,
        expected: Revision,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)>;
}

#[async_trait]
pub trait MealPlanRepository: Send + Sync + 'static {
    async fn get(&self, id: MealPlanEntryId) -> Result<Option<MealPlanEntry>>;

    async fn list(&self, query: &MealPlanQuery) -> Result<Vec<MealPlanEntry>>;

    async fn list_all(&self, from: Date, to: Date) -> Result<Vec<MealPlanEntry>>;

    async fn list_through(
        &self,
        member_id: HouseholdMemberId,
        to: Date,
    ) -> Result<Vec<MealPlanEntry>>;

    async fn list_all_through(&self, to: Date) -> Result<Vec<MealPlanEntry>>;

    async fn insert(&self, entry: &MealPlanEntry) -> Result<()>;

    async fn update(&self, entry: &MealPlanEntry, expected: Revision) -> Result<UpdateOutcome>;

    async fn delete(&self, id: MealPlanEntryId, expected: Revision) -> Result<UpdateOutcome>;

    async fn resolve(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        consumption: &[ConsumptionRecord],
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)>;

    async fn reopen(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        delete_records: &[ConsumptionRecordId],
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)>;

    async fn set_participants(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
    ) -> Result<UpdateOutcome>;

    async fn resolve_component(
        &self,
        entry_id: MealPlanEntryId,
        component: &MealPlanComponentUpdate<'_>,
        participants: &[MealParticipant],
        expected: Revision,
        consumption: Option<&ConsumptionRecord>,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)>;

    async fn reopen_component(
        &self,
        entry_id: MealPlanEntryId,
        component: &MealPlanComponentUpdate<'_>,
        participants: &[MealParticipant],
        expected: Revision,
        delete_record: Option<ConsumptionRecordId>,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)>;
}

pub enum SnapshotOp<'a> {
    Keep,
    Set(&'a MealPlanComponentSnapshot),
    Clear,
}

pub struct MealPlanComponentUpdate<'a> {
    pub id: MealPlanComponentId,
    pub snapshot: SnapshotOp<'a>,
    pub revision: Revision,
    pub actor_id: UserId,
    pub now: time::OffsetDateTime,
}

#[async_trait]
pub trait RecipeRepository: Send + Sync + 'static {
    async fn get(&self, id: RecipeId) -> Result<Option<Recipe>>;

    async fn get_many(&self, ids: &[RecipeId]) -> Result<Vec<Recipe>> {
        let mut recipes = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(recipe) = self.get(*id).await? {
                recipes.push(recipe);
            }
        }
        Ok(recipes)
    }

    async fn list(&self, query: &RecipeQuery) -> Result<Paginated<RecipeSummary>>;

    async fn referenced_ingredient_ids(
        &self,
        viewer_id: UserId,
        include_all_private: bool,
    ) -> Result<Vec<IngredientId>>;

    async fn insert(&self, recipe: &Recipe) -> Result<()>;

    async fn update(&self, recipe: &Recipe, expected: Revision) -> Result<UpdateOutcome>;

    async fn get_photo(&self, id: RecipeId) -> Result<Option<RecipePhoto>>;

    async fn update_photo(
        &self,
        recipe: &Recipe,
        expected: Revision,
        photo: Option<&RecipePhoto>,
    ) -> Result<UpdateOutcome>;
}

#[async_trait]
pub trait StockRepository: Send + Sync + 'static {
    async fn get(&self, id: StockItemId) -> Result<Option<StockItem>>;

    async fn list(&self, query: &StockQuery) -> Result<Paginated<StockItem>>;

    async fn list_for_products(&self, product_ids: &[ProductId]) -> Result<Vec<StockItem>>;

    async fn insert(&self, item: &StockItem, event: &NewStockEvent) -> Result<()>;

    async fn update(
        &self,
        item: &StockItem,
        expected: Revision,
        event: &NewStockEvent,
    ) -> Result<UpdateOutcome>;

    async fn list_events(&self, id: StockItemId) -> Result<Vec<StockEvent>>;

    async fn effects_for_source(
        &self,
        source_kind: StockEffectSource,
        source_id: uuid::Uuid,
    ) -> Result<Vec<StockEffect>>;
}

#[async_trait]
pub trait HouseholdSettingsRepository: Send + Sync + 'static {
    async fn get(&self) -> Result<HouseholdSettings>;

    async fn update(
        &self,
        settings: &HouseholdSettings,
        expected: Revision,
    ) -> Result<UpdateOutcome>;
}

#[async_trait]
pub trait NutritionTargetRepository: Send + Sync + 'static {
    async fn get(&self, id: NutritionTargetId) -> Result<Option<NutritionTarget>>;

    async fn list_for_member(&self, member_id: HouseholdMemberId) -> Result<Vec<NutritionTarget>>;

    async fn insert(&self, target: &NutritionTarget) -> Result<()>;

    async fn update(&self, target: &NutritionTarget, expected: Revision) -> Result<UpdateOutcome>;

    async fn delete(&self, id: NutritionTargetId, expected: Revision) -> Result<UpdateOutcome>;
}
