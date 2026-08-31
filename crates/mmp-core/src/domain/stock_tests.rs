use super::*;
use rust_decimal::Decimal;
use time::{Date, Month, OffsetDateTime};

fn dec(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

fn day(d: u8) -> Date {
    Date::from_calendar_date(2026, Month::August, d).unwrap()
}

fn item(level: StockLevel) -> StockItem {
    StockItem {
        id: StockItemId::new(),
        product_id: ProductId::new(),
        level,
        storage_location: StorageLocation::Chilled,
        source_date: None,
        usability_deadline: None,
        note: None,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
        archived_at: None,
    }
}

fn exact(value: i64, unit: Unit) -> StockLevel {
    StockLevel::Exact {
        quantity: Quantity::new(dec(value), unit),
    }
}

fn grams(value: i64) -> Quantity {
    Quantity::new(dec(value), Unit::Gram)
}

#[test]
fn fefo_takes_the_earliest_deadline_first_and_none_last() {
    let mut early = item(exact(500, Unit::Gram));
    early.usability_deadline = Some(UsabilityDeadline {
        date: day(25),
        basis: None,
    });
    let mut late = item(exact(500, Unit::Gram));
    late.usability_deadline = Some(UsabilityDeadline {
        date: day(28),
        basis: None,
    });
    let undated = item(exact(500, Unit::Gram));

    let DeductionPlan::Planned { takes, shortfall } =
        plan_deduction(&[undated.clone(), late.clone(), early.clone()], grams(600))
    else {
        panic!("expected a planned deduction");
    };
    assert_eq!(shortfall, Shortfall::Covered);
    assert_eq!(takes[0].stock_item_id, early.id);
    assert_eq!(takes[0].requested, grams(500));
    assert_eq!(takes[1].stock_item_id, late.id);
    assert_eq!(takes[1].requested, grams(100));
}

#[test]
fn a_take_can_span_two_items() {
    let first = item(exact(120, Unit::Gram));
    let second = item(exact(400, Unit::Gram));
    let DeductionPlan::Planned { takes, shortfall } =
        plan_deduction(&[first.clone(), second.clone()], grams(300))
    else {
        panic!("expected a planned deduction");
    };
    assert_eq!(shortfall, Shortfall::Covered);
    assert_eq!(takes.len(), 2);
    assert_eq!(takes[0].requested, grams(120));
    assert_eq!(takes[1].requested, grams(180));
}

#[test]
fn insufficient_exact_stock_is_a_definite_shortfall_and_never_goes_negative() {
    let only = item(exact(150, Unit::Gram));
    let DeductionPlan::Planned { takes, shortfall } = plan_deduction(&[only], grams(400)) else {
        panic!("expected a planned deduction");
    };
    assert_eq!(takes[0].requested, grams(150));
    assert_eq!(
        shortfall,
        Shortfall::Short {
            amount: grams(250),
            confidence: Confidence::Exact,
        }
    );
}

#[test]
fn an_incompatible_unit_candidate_makes_the_remainder_indeterminate() {
    let convertible = item(exact(100, Unit::Gram));
    let opaque = item(exact(3, Unit::Can));
    let DeductionPlan::Planned { shortfall, .. } =
        plan_deduction(&[convertible, opaque], grams(400))
    else {
        panic!("expected a planned deduction");
    };
    assert_eq!(shortfall, Shortfall::Indeterminate { amount: grams(300) });
}

#[test]
fn covering_from_an_estimated_low_bound_reports_estimated_confidence() {
    let estimated = item(StockLevel::Estimated {
        low: dec(100),
        high: dec(400),
        unit: Unit::Gram,
    });
    let DeductionPlan::Planned { takes, shortfall } = plan_deduction(&[estimated], grams(250))
    else {
        panic!("expected a planned deduction");
    };
    assert_eq!(takes[0].requested, grams(100));
    assert_eq!(
        shortfall,
        Shortfall::Short {
            amount: grams(150),
            confidence: Confidence::Estimated,
        }
    );
}

#[test]
fn a_not_tracked_item_stops_any_deduction() {
    let staple = item(StockLevel::NotTracked);
    assert_eq!(
        plan_deduction(&[staple], grams(400)),
        DeductionPlan::NotTracked
    );
}

#[test]
fn no_live_items_is_no_record() {
    let mut archived = item(exact(500, Unit::Gram));
    archived.archived_at = Some(OffsetDateTime::UNIX_EPOCH);
    assert_eq!(
        plan_deduction(&[archived], grams(400)),
        DeductionPlan::NoRecord
    );
    assert_eq!(plan_deduction(&[], grams(400)), DeductionPlan::NoRecord);
}

#[test]
fn an_estimated_band_decrements_both_bounds_floors_at_zero_and_reverses_exactly() {
    let level = StockLevel::Estimated {
        low: dec(100),
        high: dec(200),
        unit: Unit::Gram,
    };
    let applied = apply_take(&level, grams(150)).unwrap();
    assert_eq!(
        applied.new_level,
        StockLevel::Estimated {
            low: dec(0),
            high: dec(50),
            unit: Unit::Gram,
        }
    );
    assert_eq!(applied.low_delta, Some(dec(-100)));
    assert_eq!(applied.high_delta, Some(dec(-150)));

    let after = item(applied.new_level);
    let effect = StockEffect {
        id: StockEffectId::new(),
        source_kind: StockEffectSource::MealPlanComponent,
        source_id: uuid::Uuid::now_v7(),
        stock_item_id: after.id,
        product_id: after.product_id,
        state: StockEffectState::Applied,
        applied_mode: TrackingMode::Estimated,
        applied_unit: Unit::Gram,
        exact_delta: None,
        low_delta: applied.low_delta,
        high_delta: applied.high_delta,
        requested_value: dec(150),
        apply_event_id: StockEventId::new(),
        applied_at: OffsetDateTime::UNIX_EPOCH,
        released_at: None,
        note: None,
    };
    assert_eq!(
        plan_release(&after, &effect),
        ReleasePlan::Restored { new_level: level }
    );
}

#[test]
fn an_exact_take_floors_at_zero_and_reverses_to_the_amount_actually_removed() {
    let level = exact(150, Unit::Gram);
    let applied = apply_take(&level, grams(400)).unwrap();
    assert_eq!(applied.new_level, exact(0, Unit::Gram));
    assert_eq!(applied.exact_delta, Some(dec(-150)));

    let after = item(applied.new_level);
    let effect = StockEffect {
        id: StockEffectId::new(),
        source_kind: StockEffectSource::ConsumptionRecord,
        source_id: uuid::Uuid::now_v7(),
        stock_item_id: after.id,
        product_id: after.product_id,
        state: StockEffectState::Applied,
        applied_mode: TrackingMode::Exact,
        applied_unit: Unit::Gram,
        exact_delta: Some(dec(-150)),
        low_delta: None,
        high_delta: None,
        requested_value: dec(400),
        apply_event_id: StockEventId::new(),
        applied_at: OffsetDateTime::UNIX_EPOCH,
        released_at: None,
        note: None,
    };
    assert_eq!(
        plan_release(&after, &effect),
        ReleasePlan::Restored {
            new_level: exact(150, Unit::Gram),
        }
    );
}

#[test]
fn release_fails_when_the_tracking_mode_has_since_changed() {
    let now_estimated = item(StockLevel::Estimated {
        low: dec(0),
        high: dec(50),
        unit: Unit::Gram,
    });
    let effect = StockEffect {
        id: StockEffectId::new(),
        source_kind: StockEffectSource::MealPlanComponent,
        source_id: uuid::Uuid::now_v7(),
        stock_item_id: now_estimated.id,
        product_id: now_estimated.product_id,
        state: StockEffectState::Applied,
        applied_mode: TrackingMode::Exact,
        applied_unit: Unit::Gram,
        exact_delta: Some(dec(-100)),
        low_delta: None,
        high_delta: None,
        requested_value: dec(100),
        apply_event_id: StockEventId::new(),
        applied_at: OffsetDateTime::UNIX_EPOCH,
        released_at: None,
        note: None,
    };
    assert!(matches!(
        plan_release(&now_estimated, &effect),
        ReleasePlan::Failed { .. }
    ));
}

#[test]
fn exact_level_contributes_its_quantity() {
    let level = StockLevel::Exact {
        quantity: Quantity::new(dec(400), Unit::Gram),
    };
    assert_eq!(
        level.conservative_quantity(),
        Some(Quantity::new(dec(400), Unit::Gram))
    );
    assert_eq!(level.tracking_mode(), TrackingMode::Exact);
}

#[test]
fn estimated_level_contributes_its_lower_bound() {
    let level = StockLevel::Estimated {
        low: dec(100),
        high: dec(300),
        unit: Unit::Gram,
    };
    assert_eq!(
        level.conservative_quantity(),
        Some(Quantity::new(dec(100), Unit::Gram))
    );
    assert!(level.is_estimated());
}

#[test]
fn not_tracked_level_contributes_nothing_measurable() {
    let level = StockLevel::NotTracked;
    assert_eq!(level.conservative_quantity(), None);
    assert!(level.is_not_tracked());
}

#[test]
fn estimated_band_must_be_ordered() {
    let item = NewStockItem {
        product_id: ProductId::new(),
        level: StockLevel::Estimated {
            low: dec(300),
            high: dec(100),
            unit: Unit::Gram,
        },
        storage_location: StorageLocation::Chilled,
        source_date: None,
        usability_deadline: None,
        note: None,
    };
    assert!(item.validate().is_err());
}

#[test]
fn tracking_modes_round_trip_through_codes() {
    for mode in TrackingMode::ALL {
        assert_eq!(mode.code().parse::<TrackingMode>().unwrap(), mode);
    }
}

#[test]
fn a_negative_unallocated_amount_reads_as_short() {
    let short = Availability::Quantified {
        on_hand: Quantity::new(dec(1000), Unit::Gram),
        planned_demand: Quantity::new(dec(1200), Unit::Gram),
        unallocated: Quantity::new(dec(-200), Unit::Gram),
        confidence: Confidence::Exact,
    };
    assert!(short.is_short());
    assert!(!Availability::AssumedAvailable.is_short());
}
