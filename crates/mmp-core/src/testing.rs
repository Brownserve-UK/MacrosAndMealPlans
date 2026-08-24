// In-memory implementations for testing purposes
// This exists so we can test behaviour without needing to set up a database.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{
    AccessScope, ConsumptionRecord, ConsumptionRecordId, HouseholdMember, HouseholdMemberId,
    Ingredient, IngredientId, MealPlanEntry, MealPlanEntryId, MemberAccessGrant, Product,
    ProductId, Revision, Role, User, UserId,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    AccessGrantRepository, ConsumptionQuery, ConsumptionRecordRepository,
    HouseholdMemberRepository, IngredientQuery, IngredientRepository, MealPlanQuery,
    MealPlanRepository, MemberQuery, Paginated, ProductQuery, ProductRepository, SortDirection,
    UpdateOutcome, UserQuery, UserRepository,
};

// This _should_ reflect the indexs that a real database would enforce
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
}

impl InMemoryConsumptionRecordRepository {
    pub fn new() -> Self {
        Self::default()
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
        items.sort_by_key(|r| r.consumed_at);
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
        records.sort_by_key(|record| (record.consumed_at, record.id));
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
        records.sort_by_key(|record| (record.consumed_at, record.id));
        Ok(records)
    }

    async fn insert(&self, record: &ConsumptionRecord) -> Result<()> {
        self.rows.lock().unwrap().insert(record.id, record.clone());
        Ok(())
    }

    async fn update(
        &self,
        record: &ConsumptionRecord,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&record.id) {
            None => Ok(UpdateOutcome::NotFound),
            Some(existing) if existing.revision != expected => {
                Ok(UpdateOutcome::RevisionMismatch {
                    actual: existing.revision,
                })
            }
            Some(_) => {
                rows.insert(record.id, record.clone());
                Ok(UpdateOutcome::Updated)
            }
        }
    }

    async fn delete(&self, id: ConsumptionRecordId) -> Result<bool> {
        Ok(self.rows.lock().unwrap().remove(&id).is_some())
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
            .filter(|entry| entry.member_id == query.member_id)
            .filter(|entry| entry.planned_on >= query.from && entry.planned_on <= query.to)
            .cloned()
            .collect();
        entries.sort_by_key(|entry| {
            (
                entry.planned_on,
                entry.slot.order(),
                entry.planned_time,
                entry.created_at,
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
    ) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&entry.id) {
            None => return Ok(UpdateOutcome::NotFound),
            Some(current) if current.revision != expected => {
                return Ok(UpdateOutcome::RevisionMismatch {
                    actual: current.revision,
                });
            }
            Some(_) => {}
        }

        let mut records = self.consumption.rows.lock().unwrap();
        if consumption.iter().any(|candidate| {
            records.values().any(|existing| {
                candidate.meal_plan_component_id.is_some()
                    && existing.meal_plan_component_id == candidate.meal_plan_component_id
            })
        }) {
            return Err(CoreError::conflict("That meal has already been confirmed."));
        }
        for record in consumption {
            records.insert(record.id, record.clone());
        }
        rows.insert(entry.id, entry.clone());
        Ok(UpdateOutcome::Updated)
    }

    async fn reopen(&self, entry: &MealPlanEntry, expected: Revision) -> Result<UpdateOutcome> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get(&entry.id) {
            None => return Ok(UpdateOutcome::NotFound),
            Some(current) if current.revision != expected => {
                return Ok(UpdateOutcome::RevisionMismatch {
                    actual: current.revision,
                });
            }
            Some(_) => {}
        }

        let mut records = self.consumption.rows.lock().unwrap();
        records.retain(|_, record| record.meal_plan_entry_id != Some(entry.id));
        rows.insert(entry.id, entry.clone());
        Ok(UpdateOutcome::Updated)
    }
}
