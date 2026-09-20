use time::macros::time;

use super::*;

fn sample() -> MealTimes {
    MealTimes {
        breakfast: time!(08:00),
        lunch: time!(12:30),
        dinner: time!(18:00),
    }
}

#[test]
fn snacks_have_no_default_time() {
    assert_eq!(sample().for_slot(MealSlot::Snacks), None);
}

#[test]
fn timed_slots_return_their_time() {
    let times = sample();
    assert_eq!(times.for_slot(MealSlot::Breakfast), Some(time!(08:00)));
    assert_eq!(times.for_slot(MealSlot::Lunch), Some(time!(12:30)));
    assert_eq!(times.for_slot(MealSlot::Dinner), Some(time!(18:00)));
}

#[test]
fn patch_only_touches_set_fields() {
    let patch = HouseholdSettingsPatch {
        lunch_time: Some(time!(13:00)),
        ..HouseholdSettingsPatch::default()
    };
    let updated = patch.apply(sample());
    assert_eq!(updated.breakfast, time!(08:00));
    assert_eq!(updated.lunch, time!(13:00));
    assert_eq!(updated.dinner, time!(18:00));
}

#[test]
fn empty_patch_is_reported_empty() {
    assert!(HouseholdSettingsPatch::default().is_empty());
    assert!(
        !HouseholdSettingsPatch {
            dinner_time: Some(time!(19:00)),
            ..HouseholdSettingsPatch::default()
        }
        .is_empty()
    );
}
