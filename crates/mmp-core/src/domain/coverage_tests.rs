use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::date;

use super::*;
use crate::domain::{
    DemandSubject, IngredientId, MealPlanEntryId, MealPlanScope, MealSlot, ProductId, Revision,
    SourceDate, SourceDateKind, StockItemId, StockSubject, StorageLocation, Unit,
    UsabilityDeadline,
};

fn ml(value: i64) -> Quantity {
    Quantity::new(Decimal::new(value, 0), Unit::Millilitre)
}

fn item(quantity: Quantity, deadline: Option<Date>) -> StockItem {
    let now = OffsetDateTime::UNIX_EPOCH;
    StockItem {
        id: StockItemId::new(),
        subject: StockSubject::product(ProductId::new()),
        level: StockLevel::Exact { quantity },
        storage_location: StorageLocation::Chilled,
        source_date: Some(SourceDate {
            date: date!(2026 - 08 - 20),
            kind: SourceDateKind::UseBy,
        }),
        usability_deadline: deadline.map(|date| UsabilityDeadline { date, basis: None }),
        note: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

fn claim(quantity: Quantity, on: Date, assumed: bool) -> DemandClaim {
    claim_for(
        DemandSubject::ingredient(IngredientId::new()),
        quantity,
        on,
        assumed,
    )
}

fn claim_for(subject: DemandSubject, quantity: Quantity, on: Date, assumed: bool) -> DemandClaim {
    DemandClaim {
        subject,
        quantity,
        entry_id: MealPlanEntryId::new(),
        planned_on: on,
        slot: MealSlot::Dinner,
        scope: MealPlanScope::Member,
        recipe_name: None,
        assumed,
    }
}

#[test]
fn a_shortage_is_dated_by_when_it_bites_not_by_the_first_meal() {
    let stock = [item(ml(1000), None)];
    let claims = [
        claim(ml(1000), date!(2026 - 09 - 01), false),
        claim(ml(1000), date!(2026 - 09 - 04), false),
    ];

    let coverage = cover(&stock, &claims);

    assert_eq!(coverage.shortfall, Some(ml(1000)));
    assert_eq!(coverage.required_by, Some(date!(2026 - 09 - 04)));
    assert_eq!(coverage.uncovered.len(), 1);
}

#[test]
fn stock_that_goes_off_first_cannot_cover_a_later_meal() {
    let claims = [claim(ml(500), date!(2026 - 09 - 04), false)];

    let expiring = [item(ml(1000), Some(date!(2026 - 09 - 01)))];
    let coverage = cover(&expiring, &claims);
    assert_eq!(coverage.shortfall, Some(ml(500)));
    assert_eq!(coverage.required_by, Some(date!(2026 - 09 - 04)));

    let undated = [item(ml(1000), None)];
    assert!(!cover(&undated, &claims).is_short());

    let just_in_time = [item(ml(1000), Some(date!(2026 - 09 - 04)))];
    assert!(!cover(&just_in_time, &claims).is_short());
}

#[test]
fn use_by_at_least_is_the_latest_uncovered_meal_not_the_latest_meal() {
    let stock = [item(ml(500), None)];
    let claims = [
        claim(ml(500), date!(2026 - 09 - 01), false),
        claim(ml(500), date!(2026 - 09 - 03), false),
        claim(ml(500), date!(2026 - 09 - 06), false),
    ];

    let coverage = cover(&stock, &claims);

    assert_eq!(coverage.shortfall, Some(ml(1000)));
    assert_eq!(coverage.required_by, Some(date!(2026 - 09 - 03)));
    assert_eq!(coverage.use_by_at_least, Some(date!(2026 - 09 - 06)));
    assert_eq!(coverage.uncovered.len(), 2);
}

#[test]
fn a_shortage_that_only_exists_because_of_an_assumption_is_flagged() {
    let stock = [item(ml(1000), None)];
    let claims = [
        claim(ml(1000), date!(2026 - 08 - 31), true),
        claim(ml(1000), date!(2026 - 09 - 02), false),
    ];

    let coverage = cover(&stock, &claims);

    assert!(coverage.is_short());
    assert!(coverage.assumption_only);
}

#[test]
fn a_shortage_that_survives_without_the_assumption_is_not_flagged() {
    let stock = [item(ml(500), None)];
    let claims = [
        claim(ml(1000), date!(2026 - 08 - 31), true),
        claim(ml(1000), date!(2026 - 09 - 02), false),
    ];

    let coverage = cover(&stock, &claims);

    assert!(coverage.is_short());
    assert!(!coverage.assumption_only);
}

#[test]
fn covered_demand_reports_no_shortfall_and_no_dates() {
    let stock = [item(ml(2000), None)];
    let claims = [
        claim(ml(500), date!(2026 - 09 - 01), false),
        claim(ml(500), date!(2026 - 09 - 04), false),
    ];

    let coverage = cover(&stock, &claims);

    assert!(!coverage.is_short());
    assert_eq!(coverage.required_by, None);
    assert_eq!(coverage.use_by_at_least, None);
    assert!(coverage.uncovered.is_empty());
}

#[test]
fn a_not_tracked_item_is_never_short() {
    let mut assumed = item(ml(0), None);
    assumed.level = StockLevel::NotTracked;
    let claims = [claim(ml(1000), date!(2026 - 09 - 04), false)];

    assert!(!cover(&[assumed], &claims).is_short());
}

#[test]
fn a_meal_that_named_a_product_cannot_be_fed_by_a_different_one() {
    let stock = [item(ml(1000), None)];
    let claims = [claim_for(
        DemandSubject::product(ProductId::new()),
        ml(600),
        date!(2026 - 09 - 04),
        false,
    )];

    let coverage = cover(&stock, &claims);

    assert_eq!(coverage.shortfall, Some(ml(600)));
}

#[test]
fn a_meal_that_named_a_product_is_fed_by_that_product() {
    let stock = [item(ml(1000), None)];
    let named = stock[0].product_id().expect("the item is a product");
    let claims = [claim_for(
        DemandSubject::product(named),
        ml(600),
        date!(2026 - 09 - 04),
        false,
    )];

    assert!(!cover(&stock, &claims).is_short());
}

#[test]
fn the_earliest_deadline_is_spent_first() {
    let stock = [
        item(ml(500), Some(date!(2026 - 09 - 10))),
        item(ml(500), Some(date!(2026 - 09 - 02))),
    ];
    let claims = [
        claim(ml(500), date!(2026 - 09 - 01), false),
        claim(ml(500), date!(2026 - 09 - 05), false),
    ];

    assert!(!cover(&stock, &claims).is_short());
}
