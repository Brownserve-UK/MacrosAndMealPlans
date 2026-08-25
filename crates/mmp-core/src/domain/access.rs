use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Permission {
    CatalogueRead,
    CatalogueWrite,
    HouseholdRead,
    HouseholdWrite,
    AccountAdmin,
    MemberHealthData,
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
            ],
            Role::HouseholdManager => &[
                Permission::CatalogueRead,
                Permission::CatalogueWrite,
                Permission::HouseholdRead,
                Permission::HouseholdWrite,
            ],
            Role::BasicUser => &[
                Permission::CatalogueRead,
                Permission::CatalogueWrite,
                Permission::HouseholdRead,
            ],
            Role::Nutritionist => &[Permission::CatalogueRead, Permission::HouseholdRead],
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

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known role")]
pub struct UnknownRole(pub String);

impl FromStr for Role {
    type Err = UnknownRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Role::ALL
            .into_iter()
            .find(|r| r.code() == s)
            .ok_or_else(|| UnknownRole(s.to_owned()))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum AccessScope {
    HealthData,
}

impl AccessScope {
    pub const ALL: [AccessScope; 1] = [AccessScope::HealthData];

    pub const fn code(&self) -> &'static str {
        match self {
            AccessScope::HealthData => "health_data",
        }
    }

    pub const fn permission(&self) -> Permission {
        match self {
            AccessScope::HealthData => Permission::MemberHealthData,
        }
    }
}

impl fmt::Display for AccessScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known access scope")]
pub struct UnknownAccessScope(pub String);

impl FromStr for AccessScope {
    type Err = UnknownAccessScope;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AccessScope::ALL
            .into_iter()
            .find(|s2| s2.code() == s)
            .ok_or_else(|| UnknownAccessScope(s.to_owned()))
    }
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
