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
mod tests {
    use super::*;

    #[test]
    fn role_codes_round_trip() {
        for role in Role::ALL {
            assert_eq!(Role::from_str(role.code()).unwrap(), role);
        }
    }

    #[test]
    fn scope_codes_round_trip() {
        for scope in AccessScope::ALL {
            assert_eq!(AccessScope::from_str(scope.code()).unwrap(), scope);
        }
    }

    #[test]
    fn serde_representation_matches_the_role_code() {
        for role in Role::ALL {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, format!("\"{}\"", role.code()), "{role:?}");
        }
    }

    #[test]
    fn admin_holds_every_permission() {
        for role in Role::ALL {
            for permission in role.permissions() {
                assert!(
                    Role::Admin.permissions().contains(permission),
                    "admin is missing {permission}"
                );
            }
        }
    }

    #[test]
    fn only_admin_reaches_private_health_data() {
        for role in Role::ALL {
            let holds = role.permissions().contains(&Permission::MemberHealthData);
            assert_eq!(holds, role == Role::Admin, "{role:?}");
        }
    }

    #[test]
    fn managing_the_household_does_not_grant_health_data() {
        let manager = Role::HouseholdManager.permissions();
        assert!(manager.contains(&Permission::HouseholdWrite));
        assert!(!manager.contains(&Permission::MemberHealthData));
    }

    #[test]
    fn only_admin_manages_accounts() {
        for role in Role::ALL {
            let holds = role.permissions().contains(&Permission::AccountAdmin);
            assert_eq!(holds, role == Role::Admin, "{role:?}");
        }
    }

    #[test]
    fn a_basic_user_can_add_catalogue_records_but_not_manage_members() {
        let basic = Role::BasicUser.permissions();
        assert!(basic.contains(&Permission::CatalogueWrite));
        assert!(basic.contains(&Permission::HouseholdRead));
        assert!(!basic.contains(&Permission::HouseholdWrite));
    }

    #[test]
    fn permission_codes_are_unique() {
        let mut codes: Vec<&str> = Role::ALL
            .iter()
            .flat_map(|r| r.permissions())
            .map(|p| p.code())
            .collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 6);
    }
}
