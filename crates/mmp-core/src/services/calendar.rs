use std::sync::Arc;

use crate::error::Result;
use crate::ports::{Clock, HouseholdCalendar, HouseholdSettingsRepository};

pub(super) async fn household_calendar(
    settings: &dyn HouseholdSettingsRepository,
    clock: &Arc<dyn Clock>,
) -> Result<HouseholdCalendar> {
    let current = settings.get().await?;
    Ok(HouseholdCalendar::new(clock.clone(), &current.timezone))
}
