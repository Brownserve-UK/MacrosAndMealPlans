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
    assert_eq!(codes.len(), 11);
}
