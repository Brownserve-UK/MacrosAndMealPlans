use super::str_enum::str_enum;
use std::fmt;

str_enum!(AccessScope, UnknownAccessScope, "access scope");
str_enum!(Role, UnknownRole, "role");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Permission {
    CatalogueRead,
    CatalogueWrite,
    HouseholdRead,
    HouseholdWrite,
    AccountAdmin,
    MemberHealthData,
    StockRead,
    StockWrite,
    StockHistory,
    ShoppingRead,
    ShoppingWrite,
}

impl Permission {
    pub const fn code(&self) -> &'static str {
        match self {
            Permission::CatalogueRead => "catalogue:read",
            Permission::CatalogueWrite => "catalogue:write",
            Permission::HouseholdRead => "household:read",
            Permission::HouseholdWrite => "household:write",
            Permission::AccountAdmin => "account:admin",
            Permission::MemberHealthData => "member:health_data",
            Permission::StockRead => "stock:read",
            Permission::StockWrite => "stock:write",
            Permission::StockHistory => "stock:history",
            Permission::ShoppingRead => "shopping:read",
            Permission::ShoppingWrite => "shopping:write",
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    HouseholdManager,
    Nutritionist,
    BasicUser,
}

impl Role {
    pub const ALL: [Role; 4] = [
        Role::Admin,
        Role::HouseholdManager,
        Role::Nutritionist,
        Role::BasicUser,
    ];

    pub fn permissions(&self) -> &'static [Permission] {
        match self {
            Role::Admin => &[
                Permission::CatalogueRead,
                Permission::CatalogueWrite,
                Permission::HouseholdRead,
                Permission::HouseholdWrite,
                Permission::AccountAdmin,
                Permission::MemberHealthData,
                Permission::StockRead,
                Permission::StockWrite,
                Permission::StockHistory,
                Permission::ShoppingRead,
                Permission::ShoppingWrite,
            ],
            Role::HouseholdManager => &[
                Permission::CatalogueRead,
                Permission::CatalogueWrite,
                Permission::HouseholdRead,
                Permission::HouseholdWrite,
                Permission::StockRead,
                Permission::StockWrite,
                Permission::StockHistory,
                Permission::ShoppingRead,
                Permission::ShoppingWrite,
            ],
            Role::BasicUser => &[
                Permission::CatalogueRead,
                Permission::CatalogueWrite,
                Permission::HouseholdRead,
                Permission::StockRead,
                Permission::StockWrite,
                Permission::ShoppingRead,
                Permission::ShoppingWrite,
            ],
            Role::Nutritionist => &[
                Permission::CatalogueRead,
                Permission::HouseholdRead,
                Permission::StockRead,
            ],
        }
    }

    pub const fn code(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::HouseholdManager => "household_manager",
            Role::Nutritionist => "nutritionist",
            Role::BasicUser => "basic_user",
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::HouseholdManager => "Household manager",
            Role::Nutritionist => "Nutritionist",
            Role::BasicUser => "Basic user",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum AccessScope {
    HealthData,
    MealPlan,
}

impl AccessScope {
    pub const ALL: [AccessScope; 2] = [AccessScope::HealthData, AccessScope::MealPlan];

    pub const fn code(&self) -> &'static str {
        match self {
            AccessScope::HealthData => "health_data",
            AccessScope::MealPlan => "meal_plan",
        }
    }

    pub const fn permission(&self) -> Permission {
        match self {
            AccessScope::HealthData => Permission::MemberHealthData,
            AccessScope::MealPlan => Permission::HouseholdRead,
        }
    }
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
