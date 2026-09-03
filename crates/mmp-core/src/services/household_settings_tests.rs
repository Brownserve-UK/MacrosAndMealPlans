use std::sync::Arc;

use super::*;
use crate::domain::HouseholdSettingsPatch;
use crate::error::CoreError;
use crate::ports::FixedClock;
use crate::testing::InMemoryHouseholdSettingsRepository;
use time::macros::{datetime, time};

fn service() -> HouseholdSettingsService {
    HouseholdSettingsService::new(
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
        Arc::new(FixedClock::new(datetime!(2026-08-27 09:00 UTC))),
    )
}

#[tokio::test]
async fn get_returns_the_seeded_defaults() {
    let settings = service().get().await.unwrap();
    assert_eq!(settings.meal_times.breakfast, time!(08:00));
    assert_eq!(settings.meal_times.lunch, time!(12:30));
    assert_eq!(settings.meal_times.dinner, time!(18:00));
    assert_eq!(settings.revision, Revision::INITIAL);
}

#[tokio::test]
async fn update_changes_only_supplied_times_and_bumps_the_revision() {
    let service = service();
    let updated = service
        .update(
            Revision::INITIAL,
            HouseholdSettingsPatch {
                breakfast_time: Some(time!(07:15)),
                ..HouseholdSettingsPatch::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.meal_times.breakfast, time!(07:15));
    assert_eq!(updated.meal_times.lunch, time!(12:30));
    assert_eq!(updated.revision, Revision::INITIAL.next());
    assert_eq!(
        service.get().await.unwrap().meal_times.breakfast,
        time!(07:15)
    );
}

#[tokio::test]
async fn update_rejects_a_stale_revision() {
    let service = service();
    let error = service
        .update(
            Revision::new(9),
            HouseholdSettingsPatch {
                dinner_time: Some(time!(19:00)),
                ..HouseholdSettingsPatch::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn empty_patch_is_a_no_op() {
    let service = service();
    let updated = service
        .update(Revision::INITIAL, HouseholdSettingsPatch::default())
        .await
        .unwrap();
    assert_eq!(updated.revision, Revision::INITIAL);
}
