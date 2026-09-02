// In-memory implementations for testing purposes
// This exists so we can test behaviour without needing to set up a database.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use time::Date;

use crate::domain::{
    AccessScope, ConsumptionRecord, ConsumptionRecordId, HouseholdMember, HouseholdMemberId,
    HouseholdSettings, Ingredient, IngredientId, MealParticipant, MealPlanEntry, MealPlanEntryId,
    MealTimes, MemberAccessGrant, MissingStockInterpretation, NewStockEvent, NutritionTarget,
    NutritionTargetId, Product, ProductId, Quantity, Recipe, RecipeId, RecipePhoto, RecipeSummary,
    Revision, Role, StockEffect, StockEffectSource, StockEvent, StockEventId, StockItem,
    StockItemId, StockOutcome, Unit, User, UserId,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    AccessGrantRepository, ConsumptionQuery, ConsumptionRecordRepository,
    HouseholdMemberRepository, HouseholdSettingsRepository, IngredientQuery, IngredientRepository,
    MealPlanComponentUpdate, MealPlanQuery, MealPlanRepository, MemberQuery,
    NutritionTargetRepository, Paginated, ProductQuery, ProductRepository, RecipeQuery,
    RecipeRepository, SnapshotOp, SortDirection, StockQuery, StockRepository, StockWrite,
    UpdateOutcome, UserQuery, UserRepository,
};

// This _should_ reflect the indexes that a real database would enforce
// So hopefully this should catch any issues in the same way a real database would
fn enforce_ingredient_uniqueness(
    rows: &HashMap<IngredientId, Ingredient>,
    candidate: &Ingredient,
) -> Result<()> {
    for existing in rows.values() {
        if existing.id == candidate.id {
            continue;
        }
        if existing.name.eq_ignore_ascii_case(&candidate.name) {
            return Err(CoreError::duplicate("ingredient", "name", &candidate.name));
        }
        if candidate.provenance.seed_key.is_some()
            && existing.provenance.seed_key == candidate.provenance.seed_key
        {
            return Err(CoreError::duplicate("ingredient", "seed_key", ""));
        }
    }
    Ok(())
}

fn enforce_product_uniqueness(
    rows: &HashMap<ProductId, Product>,
    candidate: &Product,
) -> Result<()> {
    for existing in rows.values() {
        if existing.id == candidate.id {
            continue;
        }
        if candidate.barcode.is_some() && existing.barcode == candidate.barcode {
            return Err(CoreError::duplicate(
                "product",
                "barcode",
                candidate.barcode.clone().unwrap_or_default(),
            ));
        }
        if candidate.provenance.seed_key.is_some()
            && existing.provenance.seed_key == candidate.provenance.seed_key
        {
            return Err(CoreError::duplicate("product", "seed_key", ""));
        }
    }
    Ok(())
}

fn matches(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn paginate<T: Clone>(
    mut items: Vec<T>,
    query_page: crate::ports::PageRequest,
    sort: SortDirection,
    key: impl Fn(&T) -> String,
) -> Paginated<T> {
    items.sort_by_key(|item| key(item).to_lowercase());
    if sort == SortDirection::Descending {
        items.reverse();
    }
    let total = items.len() as i64;
    let offset = query_page.offset() as usize;
    let limit = query_page.limit() as usize;
    let page: Vec<T> = items.into_iter().skip(offset).take(limit).collect();
    Paginated::new(page, total, query_page)
}

#[derive(Default, Clone)]
pub struct InMemoryIngredientRepository {
    rows: Arc<Mutex<HashMap<IngredientId, Ingredient>>>,
    products: Arc<Mutex<Option<InMemoryProductRepository>>>,
}

impl InMemoryIngredientRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, ingredient: Ingredient) {
        self.rows.lock().unwrap().insert(ingredient.id, ingredient);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }

    pub fn link_products(&self, products: &InMemoryProductRepository) {
        *self.products.lock().unwrap() = Some(products.clone());
    }
}

#[async_trait]
impl IngredientRepository for InMemoryIngredientRepository {
    async fn get(&self, id: IngredientId) -> Result<Option<Ingredient>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Ingredient>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|i| i.name.eq_ignore_ascii_case(name))
            .cloned())
    }

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<Ingredient>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|i| i.provenance.seed_key.as_deref() == Some(seed_key))
            .cloned())
    }

    async fn list(&self, query: &IngredientQuery) -> Result<Paginated<Ingredient>> {
        let with_products: Option<std::collections::HashSet<IngredientId>> =
            query.needs_products.and_then(|_| {
                let guard = self.products.lock().unwrap();
                guard.as_ref().map(|products| {
                    let product_rows = products.rows.lock().unwrap();
                    product_rows
                        .values()
                        .filter(|p| !p.is_archived())
                        .filter_map(|p| p.mapped_ingredient_id)
                        .collect()
                })
            });

        let rows = self.rows.lock().unwrap();
        let items: Vec<Ingredient> = rows
            .values()
            .filter(|i| query.include_archived || !i.is_archived())
            .filter(|i| query.origin.is_none_or(|o| i.provenance.origin == o))
            .filter(|i| {
                query
                    .search
                    .as_deref()
                    .is_none_or(|needle| matches(&i.name, needle))
            })
            .filter(|i| match (query.needs_products, &with_products) {
                (Some(needs), Some(mapped)) => mapped.contains(&i.id) != needs,
                _ => true,
            })
            .cloned()
            .collect();
        Ok(paginate(items, query.page, query.sort, |i| i.name.clone()))
    }

    async fn insert(&self, ingredient: &Ingredient) -> Result<()> {
        let mut rows = self.rows.lock().unwrap();
        enforce_ingredient_uniqueness(&rows, ingredient)?;
        rows.insert(ingredient.id, ingredient.clone());
        Ok(())
    }

    async fn update(&self, ingredient: &Ingredient, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&ingredient.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                enforce_ingredient_uniqueness(&rows, ingredient)?;
                rows.insert(ingredient.id, ingredient.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct InMemoryProductRepository {
    rows: Arc<Mutex<HashMap<ProductId, Product>>>,
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, product: Product) {
        self.rows.lock().unwrap().insert(product.id, product);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl ProductRepository for InMemoryProductRepository {
    async fn get(&self, id: ProductId) -> Result<Option<Product>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_barcode(&self, barcode: &str) -> Result<Option<Product>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|p| p.barcode.as_deref() == Some(barcode))
            .cloned())
    }

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<Product>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|p| p.provenance.seed_key.as_deref() == Some(seed_key))
            .cloned())
    }

    async fn list(&self, query: &ProductQuery) -> Result<Paginated<Product>> {
        let rows = self.rows.lock().unwrap();
        let items: Vec<Product> = rows
            .values()
            .filter(|p| query.include_archived || !p.is_archived())
            .filter(|p| query.origin.is_none_or(|o| p.provenance.origin == o))
            .filter(|p| {
                query
                    .barcode
                    .as_deref()
                    .is_none_or(|b| p.barcode.as_deref() == Some(b))
            })
            .filter(|p| {
                query
                    .retailer
                    .as_deref()
                    .is_none_or(|r| p.retailer.as_deref().is_some_and(|pr| matches(pr, r)))
            })
            .filter(|p| {
                query
                    .mapped_ingredient_id
                    .is_none_or(|id| p.mapped_ingredient_id == Some(id))
            })
            .filter(|p| {
                query
                    .unmapped
                    .is_none_or(|unmapped| p.mapped_ingredient_id.is_none() == unmapped)
            })
            .filter(|p| {
                query
                    .search
                    .as_deref()
                    .is_none_or(|needle| matches(&p.name, needle))
            })
            .cloned()
            .collect();
        Ok(paginate(items, query.page, query.sort, |p| p.name.clone()))
    }

    async fn count_by_ingredient(
        &self,
        ingredient_ids: &[IngredientId],
    ) -> Result<std::collections::HashMap<IngredientId, i64>> {
        let rows = self.rows.lock().unwrap();
        let mut counts = std::collections::HashMap::new();
        for id in ingredient_ids {
            let count = rows
                .values()
                .filter(|p| !p.is_archived() && p.mapped_ingredient_id == Some(*id))
                .count() as i64;
            counts.insert(*id, count);
        }
        Ok(counts)
    }

    async fn list_by_ingredient(
        &self,
        ingredient_ids: &[IngredientId],
    ) -> Result<std::collections::HashMap<IngredientId, Vec<Product>>> {
        let rows = self.rows.lock().unwrap();
        let mut grouped = std::collections::HashMap::new();
        for id in ingredient_ids {
            let mut products: Vec<Product> = rows
                .values()
                .filter(|p| !p.is_archived() && p.mapped_ingredient_id == Some(*id))
                .cloned()
                .collect();
            products.sort_by_key(|product| product.name.to_lowercase());
            grouped.insert(*id, products);
        }
        Ok(grouped)
    }

    async fn insert(&self, product: &Product) -> Result<()> {
        self.rows
            .lock()
            .unwrap()
            .insert(product.id, product.clone());
        Ok(())
    }

    async fn update(&self, product: &Product, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&product.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                enforce_product_uniqueness(&rows, product)?;
                rows.insert(product.id, product.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }
}

fn enforce_member_uniqueness(
    rows: &HashMap<HouseholdMemberId, HouseholdMember>,
    candidate: &HouseholdMember,
) -> Result<()> {
    for existing in rows.values() {
        if existing.id == candidate.id {
            continue;
        }
        if existing
            .display_name
            .eq_ignore_ascii_case(&candidate.display_name)
        {
            return Err(CoreError::duplicate(
                "household member",
                "name",
                &candidate.display_name,
            ));
        }
        if candidate.linked_user_id.is_some() && existing.linked_user_id == candidate.linked_user_id
        {
            return Err(CoreError::duplicate(
                "household member",
                "linked_user_id",
                "",
            ));
        }
    }
    Ok(())
}

fn enforce_user_uniqueness(rows: &HashMap<UserId, User>, candidate: &User) -> Result<()> {
    for existing in rows.values() {
        if existing.id == candidate.id {
            continue;
        }
        if existing.username.eq_ignore_ascii_case(&candidate.username) {
            return Err(CoreError::duplicate(
                "user",
                "username",
                &candidate.username,
            ));
        }
        if candidate.auth_subject.is_some() && existing.auth_subject == candidate.auth_subject {
            return Err(CoreError::duplicate("user", "auth_subject", ""));
        }
    }
    Ok(())
}

#[derive(Default, Clone)]
pub struct InMemoryHouseholdMemberRepository {
    rows: Arc<Mutex<HashMap<HouseholdMemberId, HouseholdMember>>>,
}

impl InMemoryHouseholdMemberRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, member: HouseholdMember) {
        self.rows.lock().unwrap().insert(member.id, member);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl HouseholdMemberRepository for InMemoryHouseholdMemberRepository {
    async fn get(&self, id: HouseholdMemberId) -> Result<Option<HouseholdMember>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_display_name(&self, name: &str) -> Result<Option<HouseholdMember>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|m| m.display_name.eq_ignore_ascii_case(name))
            .cloned())
    }

    async fn find_by_linked_user(&self, user_id: UserId) -> Result<Option<HouseholdMember>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|m| m.linked_user_id == Some(user_id))
            .cloned())
    }

    async fn list(&self, query: &MemberQuery) -> Result<Paginated<HouseholdMember>> {
        let rows = self.rows.lock().unwrap();
        let items: Vec<HouseholdMember> = rows
            .values()
            .filter(|m| query.include_archived || !m.is_archived())
            .filter(|m| {
                query
                    .with_account
                    .is_none_or(|want| m.has_account() == want)
            })
            .filter(|m| {
                query
                    .search
                    .as_deref()
                    .is_none_or(|needle| matches(&m.display_name, needle))
            })
            .cloned()
            .collect();
        Ok(paginate(items, query.page, query.sort, |m| {
            m.display_name.clone()
        }))
    }

    async fn insert(&self, member: &HouseholdMember) -> Result<()> {
        let mut rows = self.rows.lock().unwrap();
        enforce_member_uniqueness(&rows, member)?;
        rows.insert(member.id, member.clone());
        Ok(())
    }

    async fn update(&self, member: &HouseholdMember, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&member.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                enforce_member_uniqueness(&rows, member)?;
                rows.insert(member.id, member.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct InMemoryUserRepository {
    rows: Arc<Mutex<HashMap<UserId, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, user: User) {
        self.rows.lock().unwrap().insert(user.id, user);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn get(&self, id: UserId) -> Result<Option<User>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .find(|u| u.username.eq_ignore_ascii_case(username))
            .cloned())
    }

    async fn list(&self, query: &UserQuery) -> Result<Paginated<User>> {
        let rows = self.rows.lock().unwrap();
        let items: Vec<User> = rows
            .values()
            .filter(|u| query.include_archived || !u.is_archived())
            .filter(|u| query.role.is_none_or(|role| u.roles.contains(&role)))
            .filter(|u| {
                query.search.as_deref().is_none_or(|needle| {
                    matches(&u.username, needle)
                        || u.display_name
                            .as_deref()
                            .is_some_and(|name| matches(name, needle))
                })
            })
            .cloned()
            .collect();
        Ok(paginate(items, query.page, query.sort, |u| {
            u.username.clone()
        }))
    }

    async fn count_with_role(&self, role: Role, include_archived: bool) -> Result<i64> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|u| include_archived || !u.is_archived())
            .filter(|u| u.roles.contains(&role))
            .count() as i64)
    }

    async fn insert(&self, user: &User) -> Result<()> {
        let mut rows = self.rows.lock().unwrap();
        enforce_user_uniqueness(&rows, user)?;
        rows.insert(user.id, user.clone());
        Ok(())
    }

    async fn update(&self, user: &User, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&user.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                enforce_user_uniqueness(&rows, user)?;
                rows.insert(user.id, user.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }
}

type GrantKey = (UserId, HouseholdMemberId, AccessScope);

#[derive(Default, Clone)]
pub struct InMemoryAccessGrantRepository {
    rows: Arc<Mutex<HashMap<GrantKey, MemberAccessGrant>>>,
}

impl InMemoryAccessGrantRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl AccessGrantRepository for InMemoryAccessGrantRepository {
    async fn list_for_member(
        &self,
        member_id: HouseholdMemberId,
    ) -> Result<Vec<MemberAccessGrant>> {
        let mut grants: Vec<MemberAccessGrant> = self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|g| g.subject_member_id == member_id)
            .cloned()
            .collect();
        grants.sort_by_key(|g| (g.grantee_user_id, g.scope));
        Ok(grants)
    }

    async fn exists(
        &self,
        grantee_user_id: UserId,
        subject_member_id: HouseholdMemberId,
        scope: AccessScope,
    ) -> Result<bool> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .contains_key(&(grantee_user_id, subject_member_id, scope)))
    }

    async fn upsert(&self, grant: &MemberAccessGrant) -> Result<()> {
        self.rows.lock().unwrap().insert(
            (grant.grantee_user_id, grant.subject_member_id, grant.scope),
            grant.clone(),
        );
        Ok(())
    }

    async fn revoke(
        &self,
        grantee_user_id: UserId,
        subject_member_id: HouseholdMemberId,
        scope: AccessScope,
    ) -> Result<bool> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .remove(&(grantee_user_id, subject_member_id, scope))
            .is_some())
    }
}

#[derive(Default, Clone)]
pub struct InMemoryConsumptionRecordRepository {
    rows: Arc<Mutex<HashMap<ConsumptionRecordId, ConsumptionRecord>>>,
    stock: InMemoryStockRepository,
}

impl InMemoryConsumptionRecordRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_stock(stock: InMemoryStockRepository) -> Self {
        Self {
            rows: Arc::new(Mutex::new(HashMap::new())),
            stock,
        }
    }

    pub fn seed(&self, record: ConsumptionRecord) {
        self.rows.lock().unwrap().insert(record.id, record);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl ConsumptionRecordRepository for InMemoryConsumptionRecordRepository {
    async fn get(&self, id: ConsumptionRecordId) -> Result<Option<ConsumptionRecord>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn list(&self, query: &ConsumptionQuery) -> Result<Paginated<ConsumptionRecord>> {
        let rows = self.rows.lock().unwrap();
        let mut items: Vec<ConsumptionRecord> = rows
            .values()
            .filter(|r| query.member_id.is_none_or(|id| r.member_id == id))
            .filter(|r| query.from.is_none_or(|from| r.consumed_on >= from))
            .filter(|r| query.to.is_none_or(|to| r.consumed_on <= to))
            .cloned()
            .collect();
        items.sort_by_key(|r| (r.created_at, r.id));
        if query.sort == SortDirection::Descending {
            items.reverse();
        }
        let total = items.len() as i64;
        let offset = query.page.offset() as usize;
        let limit = query.page.limit() as usize;
        let page: Vec<ConsumptionRecord> = items.into_iter().skip(offset).take(limit).collect();
        Ok(Paginated::new(page, total, query.page))
    }

    async fn list_period(
        &self,
        member_id: HouseholdMemberId,
        from: time::Date,
        to: time::Date,
    ) -> Result<Vec<ConsumptionRecord>> {
        let mut records: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.member_id == member_id)
            .filter(|record| record.consumed_on >= from && record.consumed_on <= to)
            .cloned()
            .collect();
        records.sort_by_key(|record| (record.created_at, record.id));
        Ok(records)
    }

    async fn list_for_meal_plan_entry(
        &self,
        entry_id: MealPlanEntryId,
    ) -> Result<Vec<ConsumptionRecord>> {
        let mut records: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.meal_plan_entry_id == Some(entry_id))
            .cloned()
            .collect();
        records.sort_by_key(|record| (record.created_at, record.id));
        Ok(records)
    }

    async fn insert(
        &self,
        record: &ConsumptionRecord,
        stock: &StockWrite,
    ) -> Result<Vec<StockOutcome>> {
        self.rows.lock().unwrap().insert(record.id, record.clone());
        Ok(self
            .stock
            .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH))
    }

    async fn update(
        &self,
        record: &ConsumptionRecord,
        expected: Revision,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        let outcome = {
            let mut rows = self.rows.lock().unwrap();
            match rows.get(&record.id) {
                None => UpdateOutcome::NotFound,
                Some(existing) if existing.revision != expected => {
                    UpdateOutcome::RevisionMismatch {
                        actual: existing.revision,
                    }
                }
                Some(_) => {
                    rows.insert(record.id, record.clone());
                    UpdateOutcome::Updated
                }
            }
        };
        if outcome == UpdateOutcome::Updated {
            let stock_outcomes = self
                .stock
                .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH);
            Ok((outcome, stock_outcomes))
        } else {
            Ok((outcome, Vec::new()))
        }
    }

    async fn delete(
        &self,
        id: ConsumptionRecordId,
        expected: Revision,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        let outcome = {
            let mut rows = self.rows.lock().unwrap();
            match rows.get(&id) {
                None => UpdateOutcome::NotFound,
                Some(existing) if existing.revision != expected => {
                    UpdateOutcome::RevisionMismatch {
                        actual: existing.revision,
                    }
                }
                Some(_) => {
                    rows.remove(&id);
                    UpdateOutcome::Updated
                }
            }
        };
        if outcome == UpdateOutcome::Updated {
            let stock_outcomes = self
                .stock
                .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH);
            Ok((outcome, stock_outcomes))
        } else {
            Ok((outcome, Vec::new()))
        }
    }
}

#[derive(Clone)]
pub struct InMemoryMealPlanRepository {
    rows: Arc<Mutex<HashMap<MealPlanEntryId, MealPlanEntry>>>,
    consumption: InMemoryConsumptionRecordRepository,
}

impl InMemoryMealPlanRepository {
    pub fn new(consumption: InMemoryConsumptionRecordRepository) -> Self {
        Self {
            rows: Arc::new(Mutex::new(HashMap::new())),
            consumption,
        }
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

impl Default for InMemoryMealPlanRepository {
    fn default() -> Self {
        Self::new(InMemoryConsumptionRecordRepository::new())
    }
}

#[async_trait]
impl MealPlanRepository for InMemoryMealPlanRepository {
    async fn get(&self, id: MealPlanEntryId) -> Result<Option<MealPlanEntry>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn list(&self, query: &MealPlanQuery) -> Result<Vec<MealPlanEntry>> {
        let mut entries: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|entry| {
                entry.member_id == Some(query.member_id)
                    || (query.include_participating
                        && (entry
                            .participants
                            .iter()
                            .any(|participant| participant.member_id == query.member_id)
                            || entry.has_opted_out(query.member_id)))
            })
            .filter(|entry| entry.planned_on >= query.from && entry.planned_on <= query.to)
            .cloned()
            .collect();
        entries.sort_by_key(|entry| {
            (
                entry.planned_on,
                entry.slot.order(),
                entry.planned_time,
                entry.created_at,
                entry.id,
            )
        });
        Ok(entries)
    }

    async fn list_all(&self, from: Date, to: Date) -> Result<Vec<MealPlanEntry>> {
        let mut entries: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|entry| entry.planned_on >= from && entry.planned_on <= to)
            .cloned()
            .collect();
        entries.sort_by_key(|entry| {
            (
                entry.planned_on,
                entry.slot.order(),
                entry.planned_time,
                entry.created_at,
                entry.id,
            )
        });
        Ok(entries)
    }

    async fn insert(&self, entry: &MealPlanEntry) -> Result<()> {
        self.rows.lock().unwrap().insert(entry.id, entry.clone());
        Ok(())
    }

    async fn update(&self, entry: &MealPlanEntry, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&entry.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(current) if current.revision != expected => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            Some(_) => {
                rows.insert(entry.id, entry.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn delete(&self, id: MealPlanEntryId, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(current) if current.revision != expected => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            Some(_) => {
                rows.remove(&id);
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn resolve(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        consumption: &[ConsumptionRecord],
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        {
            let mut rows = self.rows.lock().unwrap();
            match rows.get(&entry.id) {
                None => return Ok((UpdateOutcome::NotFound, Vec::new())),
                Some(current) if current.revision != expected => {
                    return Ok((
                        UpdateOutcome::RevisionMismatch {
                            actual: current.revision,
                        },
                        Vec::new(),
                    ));
                }
                Some(_) => {}
            }

            let mut records = self.consumption.rows.lock().unwrap();
            if consumption.iter().any(|candidate| {
                records.values().any(|existing| {
                    candidate.meal_plan_component_id.is_some()
                        && existing.meal_plan_component_id == candidate.meal_plan_component_id
                        && existing.member_id == candidate.member_id
                })
            }) {
                return Err(CoreError::conflict("That meal has already been confirmed."));
            }
            for record in consumption {
                records.insert(record.id, record.clone());
            }
            rows.insert(entry.id, entry.clone());
        }
        let stock_outcomes = self
            .consumption
            .stock
            .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH);
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }

    async fn reopen(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
        delete_records: &[ConsumptionRecordId],
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        {
            let mut rows = self.rows.lock().unwrap();
            match rows.get(&entry.id) {
                None => return Ok((UpdateOutcome::NotFound, Vec::new())),
                Some(current) if current.revision != expected => {
                    return Ok((
                        UpdateOutcome::RevisionMismatch {
                            actual: current.revision,
                        },
                        Vec::new(),
                    ));
                }
                Some(_) => {}
            }
            let mut records = self.consumption.rows.lock().unwrap();
            for record_id in delete_records {
                records.remove(record_id);
            }
            rows.insert(entry.id, entry.clone());
        }
        let stock_outcomes = self
            .consumption
            .stock
            .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH);
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }

    async fn set_participants(
        &self,
        entry: &MealPlanEntry,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&entry.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(current) if current.revision != expected => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            Some(_) => {
                rows.insert(entry.id, entry.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn resolve_component(
        &self,
        entry_id: MealPlanEntryId,
        component: &MealPlanComponentUpdate<'_>,
        participants: &[MealParticipant],
        expected: Revision,
        consumption: Option<&ConsumptionRecord>,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        {
            let mut rows = self.rows.lock().unwrap();
            let Some(entry) = rows.get_mut(&entry_id) else {
                return Ok((UpdateOutcome::NotFound, Vec::new()));
            };
            let Some(current) = entry
                .components
                .iter_mut()
                .find(|candidate| candidate.id == component.id)
            else {
                return Ok((UpdateOutcome::NotFound, Vec::new()));
            };
            if current.revision != expected {
                return Ok((
                    UpdateOutcome::RevisionMismatch {
                        actual: current.revision,
                    },
                    Vec::new(),
                ));
            }
            if let Some(record) = consumption {
                let mut records = self.consumption.rows.lock().unwrap();
                if records.values().any(|existing| {
                    existing.meal_plan_component_id == Some(component.id)
                        && existing.member_id == record.member_id
                }) {
                    return Err(CoreError::conflict("That item has already been confirmed."));
                }
                records.insert(record.id, record.clone());
            }
            apply_component_update(current, component);
            entry.participants = participants.to_vec();
            apply_entry_update(entry, component);
        }
        let stock_outcomes = self
            .consumption
            .stock
            .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH);
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }

    async fn reopen_component(
        &self,
        entry_id: MealPlanEntryId,
        component: &MealPlanComponentUpdate<'_>,
        participants: &[MealParticipant],
        expected: Revision,
        delete_record: Option<ConsumptionRecordId>,
        stock: &StockWrite,
    ) -> Result<(UpdateOutcome, Vec<StockOutcome>)> {
        {
            let mut rows = self.rows.lock().unwrap();
            let Some(entry) = rows.get_mut(&entry_id) else {
                return Ok((UpdateOutcome::NotFound, Vec::new()));
            };
            let Some(current) = entry
                .components
                .iter_mut()
                .find(|candidate| candidate.id == component.id)
            else {
                return Ok((UpdateOutcome::NotFound, Vec::new()));
            };
            if current.revision != expected {
                return Ok((
                    UpdateOutcome::RevisionMismatch {
                        actual: current.revision,
                    },
                    Vec::new(),
                ));
            }
            apply_component_update(current, component);
            entry.participants = participants.to_vec();
            apply_entry_update(entry, component);
            if let Some(record_id) = delete_record {
                self.consumption.rows.lock().unwrap().remove(&record_id);
            }
        }
        let stock_outcomes = self
            .consumption
            .stock
            .apply_write(stock, time::OffsetDateTime::UNIX_EPOCH);
        Ok((UpdateOutcome::Updated, stock_outcomes))
    }
}

fn apply_component_update(
    component: &mut crate::domain::MealPlanComponent,
    update: &MealPlanComponentUpdate<'_>,
) {
    component.revision = update.revision;
    match update.snapshot {
        SnapshotOp::Keep => {}
        SnapshotOp::Clear => component.snapshot = None,
        SnapshotOp::Set(snapshot) => {
            if component.snapshot.is_none() {
                component.snapshot = Some(snapshot.clone());
            }
        }
    }
}

fn apply_entry_update(entry: &mut MealPlanEntry, update: &MealPlanComponentUpdate<'_>) {
    entry.updated_by = update.actor_id;
    entry.updated_at = update.now;
    entry.revision = entry.revision.next();
}

fn enforce_target_uniqueness(
    rows: &HashMap<NutritionTargetId, NutritionTarget>,
    candidate: &NutritionTarget,
) -> Result<()> {
    for existing in rows.values() {
        if existing.id == candidate.id {
            continue;
        }
        if existing.member_id == candidate.member_id
            && existing.effective_from == candidate.effective_from
        {
            return Err(CoreError::duplicate(
                "nutrition target",
                "effective_from",
                candidate.effective_from,
            ));
        }
    }
    Ok(())
}

#[derive(Default, Clone)]
pub struct InMemoryNutritionTargetRepository {
    rows: Arc<Mutex<HashMap<NutritionTargetId, NutritionTarget>>>,
}

impl InMemoryNutritionTargetRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, target: NutritionTarget) {
        self.rows.lock().unwrap().insert(target.id, target);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl NutritionTargetRepository for InMemoryNutritionTargetRepository {
    async fn get(&self, id: NutritionTargetId) -> Result<Option<NutritionTarget>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn list_for_member(&self, member_id: HouseholdMemberId) -> Result<Vec<NutritionTarget>> {
        let mut targets: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .values()
            .filter(|target| target.member_id == member_id)
            .cloned()
            .collect();
        targets.sort_by_key(|target| target.effective_from);
        Ok(targets)
    }

    async fn insert(&self, target: &NutritionTarget) -> Result<()> {
        let mut rows = self.rows.lock().unwrap();
        enforce_target_uniqueness(&rows, target)?;
        rows.insert(target.id, target.clone());
        Ok(())
    }

    async fn update(&self, target: &NutritionTarget, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&target.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                enforce_target_uniqueness(&rows, target)?;
                rows.insert(target.id, target.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn delete(&self, id: NutritionTargetId, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                rows.remove(&id);
                Ok(UpdateOutcome::Updated)
            }
        }
    }
}

#[derive(Clone)]
pub struct InMemoryHouseholdSettingsRepository {
    row: Arc<Mutex<HouseholdSettings>>,
}

impl InMemoryHouseholdSettingsRepository {
    pub fn new() -> Self {
        Self {
            row: Arc::new(Mutex::new(HouseholdSettings {
                meal_times: MealTimes {
                    breakfast: time::macros::time!(08:00),
                    lunch: time::macros::time!(12:30),
                    dinner: time::macros::time!(18:00),
                },
                missing_stock_interpretation: MissingStockInterpretation::Unknown,
                default_all_members_participate: false,
                assume_eaten_when_time_passes: false,
                revision: Revision::INITIAL,
                created_at: time::OffsetDateTime::UNIX_EPOCH,
                updated_at: time::OffsetDateTime::UNIX_EPOCH,
            })),
        }
    }
}

impl InMemoryHouseholdSettingsRepository {
    pub fn set_default_all_members_participate(&self, value: bool) {
        self.row.lock().unwrap().default_all_members_participate = value;
    }

    pub fn set_assume_eaten_when_time_passes(&self, value: bool) {
        self.row.lock().unwrap().assume_eaten_when_time_passes = value;
    }

    pub fn set_meal_times(&self, meal_times: MealTimes) {
        self.row.lock().unwrap().meal_times = meal_times;
    }
}

impl Default for InMemoryHouseholdSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HouseholdSettingsRepository for InMemoryHouseholdSettingsRepository {
    async fn get(&self) -> Result<HouseholdSettings> {
        Ok(*self.row.lock().unwrap())
    }

    async fn update(
        &self,
        settings: &HouseholdSettings,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let mut row = self.row.lock().unwrap();
        if row.revision != expected {
            return Ok(UpdateOutcome::RevisionMismatch {
                actual: row.revision,
            });
        }
        *row = *settings;
        Ok(UpdateOutcome::Updated)
    }
}

#[derive(Default, Clone)]
pub struct InMemoryRecipeRepository {
    rows: Arc<Mutex<HashMap<RecipeId, Recipe>>>,
    photos: Arc<Mutex<HashMap<RecipeId, RecipePhoto>>>,
}

impl InMemoryRecipeRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, recipe: Recipe) {
        self.rows.lock().unwrap().insert(recipe.id, recipe);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }
}

#[async_trait]
impl RecipeRepository for InMemoryRecipeRepository {
    async fn get(&self, id: RecipeId) -> Result<Option<Recipe>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn list(&self, query: &RecipeQuery) -> Result<Paginated<RecipeSummary>> {
        let rows = self.rows.lock().unwrap();
        let items: Vec<RecipeSummary> = rows
            .values()
            .filter(|r| r.owner_id == query.owner_id)
            .filter(|r| query.include_archived || !r.is_archived())
            .filter(|r| {
                query
                    .search
                    .as_deref()
                    .is_none_or(|needle| matches(&r.name, needle))
            })
            .map(|recipe| RecipeSummary {
                id: recipe.id,
                name: recipe.name.clone(),
                description: recipe.description.clone(),
                servings: recipe.servings,
                preparation_minutes: recipe.preparation_minutes,
                cooking_minutes: recipe.cooking_minutes,
                component_count: recipe.components.len() as i64,
                unresolved_count: recipe
                    .components
                    .iter()
                    .filter(|component| component.requirement.is_unresolved())
                    .count() as i64,
                meal_categories: recipe.meal_categories.clone(),
                country_categories: recipe.country_categories.clone(),
                tags: recipe.tags.clone(),
                photo_version: recipe.photo_version,
                revision: recipe.revision,
                updated_at: recipe.updated_at,
                archived_at: recipe.archived_at,
            })
            .collect();
        Ok(paginate(items, query.page, query.sort, |r| r.name.clone()))
    }

    async fn insert(&self, recipe: &Recipe) -> Result<()> {
        self.rows.lock().unwrap().insert(recipe.id, recipe.clone());
        Ok(())
    }

    async fn update(&self, recipe: &Recipe, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&recipe.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                rows.insert(recipe.id, recipe.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn get_photo(&self, id: RecipeId) -> Result<Option<RecipePhoto>> {
        Ok(self.photos.lock().unwrap().get(&id).cloned())
    }

    async fn update_photo(
        &self,
        recipe: &Recipe,
        expected: Revision,
        photo: Option<&RecipePhoto>,
    ) -> Result<UpdateOutcome> {
        let outcome = self.update(recipe, expected).await?;
        if outcome == UpdateOutcome::Updated {
            let mut photos = self.photos.lock().unwrap();
            match photo {
                Some(photo) => {
                    photos.insert(recipe.id, photo.clone());
                }
                None => {
                    photos.remove(&recipe.id);
                }
            }
        }
        Ok(outcome)
    }
}

#[derive(Default, Clone)]
pub struct InMemoryStockRepository {
    rows: Arc<Mutex<HashMap<StockItemId, StockItem>>>,
    events: Arc<Mutex<Vec<StockEvent>>>,
    effects: Arc<Mutex<Vec<StockEffect>>>,
}

impl InMemoryStockRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, item: StockItem) {
        self.rows.lock().unwrap().insert(item.id, item);
    }

    pub fn count(&self) -> usize {
        self.rows.lock().unwrap().len()
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn effect_count(&self) -> usize {
        self.effects.lock().unwrap().len()
    }

    fn record(&self, item_id: StockItemId, event: &NewStockEvent) -> StockEventId {
        let id = StockEventId::new();
        self.events.lock().unwrap().push(StockEvent {
            id,
            stock_item_id: item_id,
            kind: event.kind,
            quantity_delta: event.quantity_delta,
            actor_user_id: event.actor_user_id,
            subject_member_id: event.subject_member_id,
            source: event.source.clone(),
            reverses_event_id: event.reverses_event_id,
            note: event.note.clone(),
            occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        });
        id
    }

    pub(crate) fn apply_write(
        &self,
        write: &crate::ports::StockWrite,
        now: time::OffsetDateTime,
    ) -> Vec<StockOutcome> {
        use crate::domain::{
            DeductionPlan, ReleasePlan, Shortfall, StockEffectState, StockEventKind,
            StockEventSource, apply_take, plan_deduction, plan_release,
        };

        let mut outcomes = Vec::new();

        for release in &write.releases {
            let targets: Vec<StockEffect> = self
                .effects
                .lock()
                .unwrap()
                .iter()
                .filter(|e| {
                    e.state == StockEffectState::Applied
                        && e.source_kind == release.source_kind
                        && e.source_id == release.source_id
                })
                .cloned()
                .collect();
            let mut unresolved = false;
            let mut product_id = None;
            let mut unit = Unit::Gram;
            for effect in targets {
                product_id = Some(effect.product_id);
                unit = effect.applied_unit;
                let item = self
                    .rows
                    .lock()
                    .unwrap()
                    .get(&effect.stock_item_id)
                    .cloned();
                let Some(item) = item else {
                    continue;
                };
                match plan_release(&item, &effect) {
                    ReleasePlan::Restored { new_level } => {
                        let mut updated = item.clone();
                        updated.level = new_level;
                        updated.revision = updated.revision.next();
                        updated.updated_at = now;
                        self.rows.lock().unwrap().insert(updated.id, updated);
                        let event = NewStockEvent {
                            kind: StockEventKind::Released,
                            quantity_delta: None,
                            actor_user_id: release.actor_user_id,
                            subject_member_id: release.subject_member_id,
                            source: Some(StockEventSource {
                                kind: release.source_kind,
                                id: release.source_id,
                                label: release.source_label.clone(),
                            }),
                            reverses_event_id: Some(effect.apply_event_id),
                            note: None,
                        };
                        self.record(effect.stock_item_id, &event);
                        let mut effects = self.effects.lock().unwrap();
                        if let Some(stored) = effects.iter_mut().find(|e| e.id == effect.id) {
                            stored.state = StockEffectState::Released;
                            stored.released_at = Some(now);
                        }
                    }
                    ReleasePlan::Failed { reason } => {
                        unresolved = true;
                        let event = NewStockEvent {
                            kind: StockEventKind::Released,
                            quantity_delta: None,
                            actor_user_id: release.actor_user_id,
                            subject_member_id: release.subject_member_id,
                            source: Some(StockEventSource {
                                kind: release.source_kind,
                                id: release.source_id,
                                label: release.source_label.clone(),
                            }),
                            reverses_event_id: Some(effect.apply_event_id),
                            note: Some(reason.clone()),
                        };
                        self.record(effect.stock_item_id, &event);
                        let mut effects = self.effects.lock().unwrap();
                        if let Some(stored) = effects.iter_mut().find(|e| e.id == effect.id) {
                            stored.state = StockEffectState::ReleaseFailed;
                            stored.note = Some(reason.clone());
                        }
                    }
                }
            }
            if unresolved && let Some(product_id) = product_id {
                outcomes.push(StockOutcome {
                    product_id,
                    wanted: Quantity::new(rust_decimal::Decimal::ZERO, unit),
                    deducted: Quantity::new(rust_decimal::Decimal::ZERO, unit),
                    shortfall: Shortfall::Covered,
                    unresolved_release: true,
                });
            }
        }

        for deduction in &write.deductions {
            let items: Vec<StockItem> = {
                let rows = self.rows.lock().unwrap();
                rows.values()
                    .filter(|item| item.product_id == deduction.product_id)
                    .cloned()
                    .collect()
            };
            let DeductionPlan::Planned { takes, shortfall } =
                plan_deduction(&items, deduction.want)
            else {
                continue;
            };

            let mut deducted = rust_decimal::Decimal::ZERO;
            for take in &takes {
                let already = self.effects.lock().unwrap().iter().any(|e| {
                    e.state == StockEffectState::Applied
                        && e.source_kind == deduction.source_kind
                        && e.source_id == deduction.source_id
                        && e.stock_item_id == take.stock_item_id
                });
                if already {
                    continue;
                }
                let mut rows = self.rows.lock().unwrap();
                let Some(item) = rows.get(&take.stock_item_id).cloned() else {
                    continue;
                };
                let Some(applied) = apply_take(&item.level, take.requested) else {
                    continue;
                };
                let mut updated = item.clone();
                updated.level = applied.new_level;
                updated.revision = updated.revision.next();
                updated.updated_at = now;
                rows.insert(updated.id, updated.clone());
                drop(rows);

                let delta = applied
                    .exact_delta
                    .or(applied.low_delta)
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let event = NewStockEvent {
                    kind: StockEventKind::Consumed,
                    quantity_delta: Some(Quantity::new(delta, take.requested.unit)),
                    actor_user_id: deduction.actor_user_id,
                    subject_member_id: deduction.subject_member_id,
                    source: Some(StockEventSource {
                        kind: deduction.source_kind,
                        id: deduction.source_id,
                        label: deduction.source_label.clone(),
                    }),
                    reverses_event_id: None,
                    note: None,
                };
                let event_id = self.record(take.stock_item_id, &event);
                self.effects.lock().unwrap().push(StockEffect {
                    id: crate::domain::StockEffectId::new(),
                    source_kind: deduction.source_kind,
                    source_id: deduction.source_id,
                    stock_item_id: take.stock_item_id,
                    product_id: deduction.product_id,
                    state: StockEffectState::Applied,
                    applied_mode: item.tracking_mode(),
                    applied_unit: take.requested.unit,
                    exact_delta: applied.exact_delta,
                    low_delta: applied.low_delta,
                    high_delta: applied.high_delta,
                    requested_value: take.requested.amount,
                    apply_event_id: event_id,
                    applied_at: now,
                    released_at: None,
                    note: None,
                });
                if let Ok(converted) = take.requested.convert_to(deduction.want.unit) {
                    deducted += converted.amount;
                }
            }

            if !matches!(shortfall, Shortfall::Covered) {
                outcomes.push(StockOutcome {
                    product_id: deduction.product_id,
                    wanted: deduction.want,
                    deducted: Quantity::new(deducted, deduction.want.unit),
                    shortfall,
                    unresolved_release: false,
                });
            }
        }

        outcomes
    }
}

#[async_trait]
impl StockRepository for InMemoryStockRepository {
    async fn get(&self, id: StockItemId) -> Result<Option<StockItem>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn list(&self, query: &StockQuery) -> Result<Paginated<StockItem>> {
        let rows = self.rows.lock().unwrap();
        let items: Vec<StockItem> = rows
            .values()
            .filter(|item| query.include_archived || !item.is_archived())
            .filter(|item| query.product_id.is_none_or(|id| item.product_id == id))
            .cloned()
            .collect();
        Ok(paginate(items, query.page, query.sort, |item| {
            item.id.to_string()
        }))
    }

    async fn list_for_products(&self, product_ids: &[ProductId]) -> Result<Vec<StockItem>> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|item| product_ids.contains(&item.product_id))
            .cloned()
            .collect())
    }

    async fn insert(&self, item: &StockItem, event: &NewStockEvent) -> Result<()> {
        self.rows.lock().unwrap().insert(item.id, item.clone());
        self.record(item.id, event);
        Ok(())
    }

    async fn update(
        &self,
        item: &StockItem,
        expected: Revision,
        event: &NewStockEvent,
    ) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&item.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                rows.insert(item.id, item.clone());
                drop(rows);
                self.record(item.id, event);
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn list_events(&self, id: StockItemId) -> Result<Vec<StockEvent>> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.stock_item_id == id)
            .cloned()
            .collect())
    }

    async fn effects_for_source(
        &self,
        source_kind: StockEffectSource,
        source_id: uuid::Uuid,
    ) -> Result<Vec<StockEffect>> {
        Ok(self
            .effects
            .lock()
            .unwrap()
            .iter()
            .filter(|effect| effect.source_kind == source_kind && effect.source_id == source_id)
            .cloned()
            .collect())
    }
}
