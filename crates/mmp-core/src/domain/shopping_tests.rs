use time::OffsetDateTime;
use time::macros::date;

use super::*;
use crate::domain::Revision;

fn cadence(interval_weeks: u8, days: Vec<Weekday>, anchor: Date) -> ShoppingCadence {
    ShoppingCadence {
        interval_weeks,
        days,
        anchor,
        usual_time: None,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn exception(
    state: ExceptionState,
    generated_for: Option<Date>,
    effective_date: Option<Date>,
) -> OpportunityException {
    OpportunityException {
        id: ShoppingOpportunityId::new(),
        generated_for,
        effective_date,
        usual_time: None,
        state,
        note: None,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

#[test]
fn shopping_every_day_lands_on_every_day() {
    let every_day = cadence(
        1,
        vec![
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ],
        date!(2026 - 08 - 31),
    );

    let dates = every_day.occurrences(date!(2026 - 08 - 31), date!(2026 - 09 - 06));

    assert_eq!(dates.len(), 7);
}

#[test]
fn twice_a_week_lands_on_both_chosen_days() {
    let twice = cadence(
        1,
        vec![Weekday::Wednesday, Weekday::Saturday],
        date!(2026 - 08 - 31),
    );

    let dates = twice.occurrences(date!(2026 - 08 - 31), date!(2026 - 09 - 13));

    assert_eq!(
        dates,
        vec![
            date!(2026 - 09 - 02),
            date!(2026 - 09 - 05),
            date!(2026 - 09 - 09),
            date!(2026 - 09 - 12),
        ]
    );
}

#[test]
fn a_fortnightly_shop_skips_the_intervening_week() {
    let fortnightly = cadence(2, vec![Weekday::Saturday], date!(2026 - 08 - 31));

    let dates = fortnightly.occurrences(date!(2026 - 08 - 31), date!(2026 - 09 - 27));

    assert_eq!(dates, vec![date!(2026 - 09 - 05), date!(2026 - 09 - 19)]);
}

#[test]
fn a_fortnightly_shop_keeps_its_cycle_before_the_anchor_week() {
    let fortnightly = cadence(2, vec![Weekday::Saturday], date!(2026 - 09 - 14));

    let dates = fortnightly.occurrences(date!(2026 - 08 - 24), date!(2026 - 09 - 20));

    assert_eq!(dates, vec![date!(2026 - 09 - 05), date!(2026 - 09 - 19)]);
}

#[test]
fn a_skipped_shop_disappears_and_a_moved_one_sorts_where_it_landed() {
    let weekly = cadence(1, vec![Weekday::Saturday], date!(2026 - 08 - 31));
    let exceptions = vec![
        exception(ExceptionState::Skipped, Some(date!(2026 - 09 - 05)), None),
        exception(
            ExceptionState::Moved,
            Some(date!(2026 - 09 - 12)),
            Some(date!(2026 - 09 - 10)),
        ),
    ];

    let opportunities = expand_opportunities(
        Some(&weekly),
        &exceptions,
        date!(2026 - 09 - 01),
        date!(2026 - 09 - 20),
    );

    let dates: Vec<Date> = opportunities.iter().map(|o| o.date).collect();
    assert_eq!(dates, vec![date!(2026 - 09 - 10), date!(2026 - 09 - 19)]);
    assert_eq!(opportunities[0].state, OpportunityState::Moved);
}

#[test]
fn a_one_off_shop_takes_part_alongside_the_cadence() {
    let weekly = cadence(1, vec![Weekday::Saturday], date!(2026 - 08 - 31));
    let exceptions = vec![exception(
        ExceptionState::OneOff,
        None,
        Some(date!(2026 - 09 - 02)),
    )];

    let opportunities = expand_opportunities(
        Some(&weekly),
        &exceptions,
        date!(2026 - 09 - 01),
        date!(2026 - 09 - 07),
    );

    let dates: Vec<Date> = opportunities.iter().map(|o| o.date).collect();
    assert_eq!(dates, vec![date!(2026 - 09 - 02), date!(2026 - 09 - 05)]);
}

#[test]
fn with_no_cadence_we_invent_nothing() {
    let opportunities =
        expand_opportunities(None, &[], date!(2026 - 09 - 01), date!(2026 - 09 - 30));

    assert!(opportunities.is_empty());
}

fn opportunity(date: Date) -> ShoppingOpportunity {
    ShoppingOpportunity {
        id: None,
        date,
        state: OpportunityState::Normal,
        generated_for: None,
        usual_time: None,
        note: None,
    }
}

#[test]
fn a_requirement_goes_to_the_last_shop_that_still_gets_it_home_in_time() {
    let shops = [
        opportunity(date!(2026 - 09 - 02)),
        opportunity(date!(2026 - 09 - 05)),
        opportunity(date!(2026 - 09 - 09)),
    ];

    assert_eq!(
        assign(Some(date!(2026 - 09 - 07)), &shops),
        Assignment::Opportunity {
            date: date!(2026 - 09 - 05)
        }
    );
}

#[test]
fn a_requirement_needed_before_any_shop_asks_for_an_earlier_one() {
    let shops = [opportunity(date!(2026 - 09 - 05))];

    assert_eq!(
        assign(Some(date!(2026 - 09 - 03)), &shops),
        Assignment::NeedsEarlierOpportunity
    );
}

#[test]
fn with_no_shops_at_all_a_requirement_stays_unassigned() {
    assert_eq!(
        assign(Some(date!(2026 - 09 - 03)), &[]),
        Assignment::Unassigned
    );
}

#[test]
fn a_shop_on_the_day_itself_still_counts() {
    let shops = [opportunity(date!(2026 - 09 - 05))];

    assert_eq!(
        assign(Some(date!(2026 - 09 - 05)), &shops),
        Assignment::Opportunity {
            date: date!(2026 - 09 - 05)
        }
    );
}
