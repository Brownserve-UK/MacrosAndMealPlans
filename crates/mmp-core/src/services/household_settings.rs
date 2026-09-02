use std::sync::Arc;

use crate::domain::{HouseholdSettings, HouseholdSettingsPatch, Revision};
use crate::error::{CoreError, Result};
use crate::ports::{Clock, HouseholdSettingsRepository, UpdateOutcome};

const HOUSEHOLD_SETTINGS: &str = "household settings";
const HOUSEHOLD_ID: &str = "household";

#[derive(Clone)]
pub struct HouseholdSettingsService {
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl HouseholdSettingsService {
    pub fn new(settings: Arc<dyn HouseholdSettingsRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { settings, clock }
    }

    pub async fn get(&self) -> Result<HouseholdSettings> {
        self.settings.get().await
    }

    pub async fn update(
        &self,
        expected: Revision,
        patch: HouseholdSettingsPatch,
    ) -> Result<HouseholdSettings> {
        let mut current = self.settings.get().await?;
        require_revision(expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        current.meal_times = patch.apply(current.meal_times);
        if let Some(interpretation) = patch.missing_stock_interpretation {
            current.missing_stock_interpretation = interpretation;
        }
        if let Some(participate) = patch.default_all_members_participate {
            current.default_all_members_participate = participate;
        }
        if let Some(assume) = patch.assume_eaten_when_time_passes {
            current.assume_eaten_when_time_passes = assume;
        }
        current.revision = current.revision.next();
        current.updated_at = self.clock.now();
        commit_outcome(self.settings.update(&current, expected).await?, expected)?;
        Ok(current)
    }
}

fn require_revision(expected: Revision, actual: Revision) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: HOUSEHOLD_SETTINGS,
            id: HOUSEHOLD_ID.to_owned(),
            expected,
            actual,
        })
    }
}

fn commit_outcome(outcome: UpdateOutcome, expected: Revision) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource: HOUSEHOLD_SETTINGS,
            id: HOUSEHOLD_ID.to_owned(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(HOUSEHOLD_SETTINGS, HOUSEHOLD_ID)),
    }
}

#[cfg(test)]
#[path = "household_settings_tests.rs"]
mod tests;
