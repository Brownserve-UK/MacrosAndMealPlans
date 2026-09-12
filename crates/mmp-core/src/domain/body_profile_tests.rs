use super::*;
use time::macros::{date, datetime};

fn profile() -> MemberBodyProfile {
    MemberBodyProfile {
        member_id: HouseholdMemberId::seeded("profile"),
        date_of_birth: Some(date!(1985 - 09 - 13)),
        sex: Some(Sex::Male),
        height_cm: Some(Decimal::new(178, 0)),
        habitual_activity: Some(HabitualActivity::LightlyActive),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-12 09:00 UTC),
        updated_at: datetime!(2026-09-12 09:00 UTC),
    }
}

#[test]
fn age_accounts_for_whether_the_birthday_has_passed() {
    let born = date!(1985 - 09 - 13);
    assert_eq!(age_on(born, date!(2026 - 09 - 12)).unwrap(), 40);
    assert_eq!(age_on(born, date!(2026 - 09 - 13)).unwrap(), 41);
}

#[test]
fn a_future_date_of_birth_is_rejected() {
    let error = age_on(date!(2027 - 01 - 01), date!(2026 - 09 - 12)).unwrap_err();
    assert!(matches!(error, crate::error::CoreError::Validation(_)));
}

#[test]
fn an_implausible_height_is_rejected() {
    let mut candidate = profile();
    candidate.height_cm = Some(Decimal::new(50, 0));
    assert!(candidate.validate(date!(2026 - 09 - 12)).is_err());
}

#[test]
fn a_patch_can_set_clear_and_preserve_profile_fields() {
    let mut candidate = profile();
    MemberBodyProfilePatch {
        date_of_birth: Patch::Clear,
        height_cm: Patch::Set(Decimal::new(180, 0)),
        ..Default::default()
    }
    .apply(&mut candidate);

    assert_eq!(candidate.date_of_birth, None);
    assert_eq!(candidate.height_cm, Some(Decimal::new(180, 0)));
    assert_eq!(candidate.sex, Some(Sex::Male));
}
