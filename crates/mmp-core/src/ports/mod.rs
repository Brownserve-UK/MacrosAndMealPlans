mod clock;
mod repository;

pub use clock::{Clock, FixedClock, SystemClock};
pub use repository::{
    AccessGrantRepository, ConsumptionQuery, ConsumptionRecordRepository,
    HouseholdMemberRepository, HouseholdSettingsRepository, IngredientQuery, IngredientRepository,
    MealPlanComponentUpdate, MealPlanQuery, MealPlanRepository, MemberQuery,
    NutritionTargetRepository, ProductQuery, ProductRepository, RecipeQuery, RecipeRepository,
    SnapshotOp, SortDirection, StockDeduction, StockQuery, StockRelease, StockRepository,
    StockWrite, UpdateOutcome, UserQuery, UserRepository,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    page: u32,
    per_page: u32,
}

impl PageRequest {
    pub const DEFAULT_PER_PAGE: u32 = 25;
    pub const MAX_PER_PAGE: u32 = 200;

    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, Self::MAX_PER_PAGE),
        }
    }

    pub const fn page(&self) -> u32 {
        self.page
    }

    pub const fn per_page(&self) -> u32 {
        self.per_page
    }

    pub fn limit(&self) -> i64 {
        i64::from(self.per_page)
    }

    pub fn offset(&self) -> i64 {
        i64::from(self.page - 1) * i64::from(self.per_page)
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self::new(1, Self::DEFAULT_PER_PAGE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

impl<T> Paginated<T> {
    pub fn new(items: Vec<T>, total: i64, request: PageRequest) -> Self {
        Self {
            items,
            total,
            page: request.page(),
            per_page: request.per_page(),
        }
    }

    pub fn total_pages(&self) -> u32 {
        if self.total <= 0 {
            return 0;
        }
        let per_page = i64::from(self.per_page);
        u32::try_from((self.total + per_page - 1) / per_page).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
