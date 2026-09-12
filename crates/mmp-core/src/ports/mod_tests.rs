use std::sync::Arc;

use time::macros::{date, datetime};

use super::*;

#[test]
fn a_household_calendar_rolls_the_date_over_at_local_midnight_not_utc_midnight() {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(datetime!(2026-06-15 23:30 UTC)));
    let calendar = HouseholdCalendar::new(clock, "Europe/London");
    assert_eq!(calendar.today(), date!(2026 - 06 - 16));
}

#[test]
fn a_household_calendar_in_utc_matches_the_clock_exactly() {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(datetime!(2026-06-15 23:30 UTC)));
    let calendar = HouseholdCalendar::new(clock, DEFAULT_TIMEZONE);
    assert_eq!(calendar.today(), date!(2026 - 06 - 15));
}

#[test]
fn an_unknown_timezone_name_falls_back_to_utc() {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(datetime!(2026-06-15 23:30 UTC)));
    let calendar = HouseholdCalendar::new(clock, "Not/A_Zone");
    assert_eq!(calendar.today(), date!(2026 - 06 - 15));
}

#[test]
fn page_requests_are_clamped() {
    let request = PageRequest::new(0, 10_000);
    assert_eq!(request.page(), 1);
    assert_eq!(request.per_page(), PageRequest::MAX_PER_PAGE);
}

#[test]
fn offsets_are_zero_based() {
    assert_eq!(PageRequest::new(1, 25).offset(), 0);
    assert_eq!(PageRequest::new(3, 25).offset(), 50);
}

#[test]
fn total_pages_rounds_up() {
    let page: Paginated<()> = Paginated::new(vec![], 51, PageRequest::new(1, 25));
    assert_eq!(page.total_pages(), 3);
}

#[test]
fn an_empty_result_has_no_pages() {
    let page: Paginated<()> = Paginated::new(vec![], 0, PageRequest::default());
    assert_eq!(page.total_pages(), 0);
}
