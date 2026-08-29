use super::*;
use rust_decimal::Decimal;

fn dec(value: i64) -> Decimal {
    Decimal::new(value, 0)
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
