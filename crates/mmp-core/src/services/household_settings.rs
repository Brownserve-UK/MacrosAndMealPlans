use std::sync::Arc;

use super::revision::{commit_outcome, require_revision};
use crate::domain::{HouseholdSettings, HouseholdSettingsPatch, Revision};
use crate::error::Result;
use crate::ports::{Clock, HouseholdSettingsRepository};

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
        require_revision(HOUSEHOLD_SETTINGS, HOUSEHOLD_ID, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }
        patch.validate()?;

        if let Some(order) = &patch.section_order {
            current.section_order = crate::domain::SectionOrder::from_slice(order)?;
        }
        if let Some(timezone) = &patch.timezone {
            current.timezone = timezone.clone();
        }
        current.meal_times = patch.clone().apply(current.meal_times);
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
        commit_outcome(
            HOUSEHOLD_SETTINGS,
            HOUSEHOLD_ID,
            expected,
            self.settings.update(&current, expected).await?,
        )?;
        Ok(current)
    }
}

#[cfg(test)]
#[path = "household_settings_tests.rs"]
mod tests;
