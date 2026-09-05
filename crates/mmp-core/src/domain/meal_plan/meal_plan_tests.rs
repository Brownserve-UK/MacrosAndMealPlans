use super::*;
use crate::domain::{MealItemRef, NutritionFacts, ProductId, Quantity, RecipeId, Unit};
use rust_decimal::Decimal;
use time::OffsetDateTime;

fn servings(value: i64) -> ConsumedAmount {
    ConsumedAmount::Servings(Decimal::new(value, 0))
}

fn grams(value: i64) -> ConsumedAmount {
    ConsumedAmount::Measure(Quantity::new(Decimal::new(value, 0), Unit::Gram))
}

fn allocation(
    component_id: MealPlanComponentId,
    allocated: ConsumedAmount,
) -> MealParticipantAllocation {
    MealParticipantAllocation {
        id: MealParticipantAllocationId::new(),
        component_id,
        allocated,
        status: ParticipantStatus::Planned,
        consumption_record_id: None,
        resolved_by: None,
        resolved_at: None,
    }
}

fn guest_alloc(
    component_id: MealPlanComponentId,
    allocated: ConsumedAmount,
    status: ParticipantStatus,
) -> MealGuestAllocation {
    MealGuestAllocation {
        id: MealGuestAllocationId::new(),
        component_id,
        allocated,
        status,
        confirmed: None,
        resolved_by: None,
        resolved_at: None,
    }
}

fn guest_group(allocations: Vec<MealGuestAllocation>) -> MealGuestGroup {
    MealGuestGroup {
        id: MealGuestGroupId::new(),
        count: 1,
        allocations,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn participant(allocations: Vec<MealParticipantAllocation>) -> MealParticipant {
    MealParticipant {
        id: MealParticipantId::new(),
        member_id: HouseholdMemberId::new(),
        allocations,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn resolved(
    mut alloc: MealParticipantAllocation,
    status: ParticipantStatus,
) -> MealParticipantAllocation {
    alloc.status = status;
    alloc
}

fn outcome(
    allocated: ConsumedAmount,
    status: ParticipantStatus,
    confirmed: Option<ConsumedAmount>,
) -> AllocationOutcome {
    AllocationOutcome {
        allocated,
        status,
        confirmed,
    }
}

fn component(id: MealPlanComponentId, amount: ConsumedAmount) -> MealPlanComponent {
    MealPlanComponent {
        id,
        item: MealItemRef::product(ProductId::new()),
        amount,
        position: 0,
        snapshot: None,
        revision: Revision::INITIAL,
        display_order: uuid::Uuid::nil(),
    }
}

#[test]
fn slot_codes_round_trip() {
    for slot in MealSlot::ALL {
        assert_eq!(MealSlot::from_str(slot.code()).unwrap(), slot);
    }
}

#[test]
fn status_codes_round_trip() {
    for status in MealPlanStatus::ALL {
        assert_eq!(MealPlanStatus::from_str(status.code()).unwrap(), status);
    }
}

#[test]
fn a_meal_needs_a_component() {
    assert!(validate_components(&[]).is_err());
}

#[test]
fn a_component_needs_a_positive_amount() {
    let components = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: ConsumedAmount::Servings(Decimal::ZERO),
    }];
    assert!(validate_components(&components).is_err());
}

#[test]
fn a_recipe_component_must_be_measured_in_servings() {
    let grams = ConsumedAmount::Measure(crate::domain::Quantity::new(
        Decimal::new(100, 0),
        crate::domain::Unit::Gram,
    ));
    let recipe_component = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::recipe(RecipeId::new()),
        amount: grams,
    }];
    assert!(validate_components(&recipe_component).is_err());

    let product_component = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: grams,
    }];
    assert!(validate_components(&product_component).is_ok());
}

#[test]
fn scope_codes_round_trip() {
    for scope in MealPlanScope::ALL {
        assert_eq!(MealPlanScope::from_str(scope.code()).unwrap(), scope);
    }
}

#[test]
fn participant_status_codes_round_trip() {
    for status in ParticipantStatus::ALL {
        assert_eq!(ParticipantStatus::from_str(status.code()).unwrap(), status);
    }
}

#[test]
fn leftover_is_prepared_minus_allocated_for_servings() {
    let prepared = servings(4);
    let outcomes = vec![
        outcome(servings(1), ParticipantStatus::Planned, None),
        outcome(servings(1), ParticipantStatus::Planned, None),
        outcome(servings(1), ParticipantStatus::Planned, None),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.allocated, Some(servings(3)));
    assert_eq!(prep.unallocated, Some(servings(1)));
    assert_eq!(prep.leftover, Some(servings(1)));
    assert!(!prep.shortage);
}

#[test]
fn leftover_is_prepared_minus_allocated_for_measures() {
    let prepared = grams(300);
    let outcomes = vec![
        outcome(grams(100), ParticipantStatus::Planned, None),
        outcome(grams(100), ParticipantStatus::Planned, None),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.allocated, Some(grams(200)));
    assert_eq!(prep.leftover, Some(grams(100)));
    assert!(!prep.shortage);
}

#[test]
fn over_allocation_is_flagged_as_a_shortage() {
    let prepared = servings(3);
    let outcomes = vec![
        outcome(servings(2), ParticipantStatus::Planned, None),
        outcome(servings(1), ParticipantStatus::Planned, None),
        outcome(servings(1), ParticipantStatus::Planned, None),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.allocated, Some(servings(4)));
    assert_eq!(prep.unallocated, Some(servings(0)));
    assert!(prep.shortage);
}

#[test]
fn a_mixed_kind_allocation_yields_no_total() {
    let prepared = servings(4);
    let outcomes = vec![
        outcome(servings(1), ParticipantStatus::Planned, None),
        outcome(grams(100), ParticipantStatus::Planned, None),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.allocated, None);
    assert_eq!(prep.leftover, None);
    assert!(!prep.shortage);
}

#[test]
fn eating_the_spare_serving_drives_leftover_to_zero() {
    let prepared = servings(4);
    let outcomes = vec![
        outcome(servings(1), ParticipantStatus::Eaten, Some(servings(2))),
        outcome(servings(1), ParticipantStatus::Eaten, Some(servings(1))),
        outcome(servings(1), ParticipantStatus::Eaten, Some(servings(1))),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.unallocated, Some(servings(1)));
    assert_eq!(prep.leftover, Some(servings(0)));
}

#[test]
fn a_declined_participant_consumes_nothing() {
    let prepared = servings(3);
    let outcomes = vec![
        outcome(servings(1), ParticipantStatus::Eaten, Some(servings(1))),
        outcome(servings(1), ParticipantStatus::NotEaten, None),
        outcome(servings(1), ParticipantStatus::Planned, None),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.leftover, Some(servings(1)));
}

#[test]
fn leftovers_are_not_inferred_from_participant_count() {
    let prepared = servings(6);
    let outcomes = vec![
        outcome(servings(1), ParticipantStatus::Planned, None),
        outcome(servings(1), ParticipantStatus::Planned, None),
    ];
    let prep = preparation_for(&prepared, &outcomes);
    assert_eq!(prep.leftover, Some(servings(4)));
}

#[test]
fn participant_status_rolls_up_across_components() {
    let one = MealPlanComponentId::new();
    let two = MealPlanComponentId::new();

    let all_pending = participant(vec![
        allocation(one, servings(1)),
        allocation(two, servings(1)),
    ]);
    assert_eq!(
        derive_participant_status(&all_pending, Assumption::NONE),
        MealPlanStatus::Planned
    );

    let one_eaten = participant(vec![
        resolved(allocation(one, servings(1)), ParticipantStatus::Eaten),
        allocation(two, servings(1)),
    ]);
    assert_eq!(
        derive_participant_status(&one_eaten, Assumption::NONE),
        MealPlanStatus::PartiallyResolved
    );

    let eaten_and_declined = participant(vec![
        resolved(allocation(one, servings(1)), ParticipantStatus::Eaten),
        resolved(allocation(two, servings(1)), ParticipantStatus::NotEaten),
    ]);
    assert_eq!(
        derive_participant_status(&eaten_and_declined, Assumption::NONE),
        MealPlanStatus::Eaten
    );

    let all_declined = participant(vec![
        resolved(allocation(one, servings(1)), ParticipantStatus::NotEaten),
        resolved(allocation(two, servings(1)), ParticipantStatus::NotEaten),
    ]);
    assert_eq!(
        derive_participant_status(&all_declined, Assumption::NONE),
        MealPlanStatus::NotEaten
    );
}

#[test]
fn component_status_rolls_up_across_participants() {
    let comp = MealPlanComponentId::new();
    let ate = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::Eaten,
    )]);
    let pending = participant(vec![allocation(comp, servings(1))]);
    assert_eq!(
        derive_component_status(comp, &[ate.clone(), pending], &[], Assumption::NONE),
        MealPlanStatus::PartiallyResolved
    );

    let declined = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::NotEaten,
    )]);
    assert_eq!(
        derive_component_status(comp, &[ate.clone(), declined], &[], Assumption::NONE),
        MealPlanStatus::Eaten
    );

    let pending_guest = guest_group(vec![guest_alloc(
        comp,
        servings(1),
        ParticipantStatus::Planned,
    )]);
    assert_eq!(
        derive_component_status(comp, &[ate], &[pending_guest], Assumption::NONE),
        MealPlanStatus::PartiallyResolved
    );
}

#[test]
fn equal_split_divides_every_amount_kind() {
    assert_eq!(equal_split(&servings(4), 4), servings(1));
    assert_eq!(equal_split(&grams(500), 4), grams(125));
    assert_eq!(
        equal_split(&ConsumedAmount::Packs(Decimal::new(3, 0)), 2),
        ConsumedAmount::Packs(Decimal::new(15, 1))
    );
    assert_eq!(equal_split(&servings(2), 0), servings(2));
}

#[test]
fn entry_is_resolved_only_when_every_participant_is() {
    let comp = MealPlanComponentId::new();

    let ate = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::Eaten,
    )]);
    let declined = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::NotEaten,
    )]);
    let pending = participant(vec![allocation(comp, servings(1))]);

    assert_eq!(
        derive_entry_status(&[ate.clone(), pending], &[], Assumption::NONE),
        MealPlanStatus::PartiallyResolved
    );
    assert_eq!(
        derive_entry_status(&[ate, declined], &[], Assumption::NONE),
        MealPlanStatus::Eaten
    );
}

#[test]
fn snapshot_scales_to_a_participants_confirmed_servings() {
    let snapshot = MealPlanComponentSnapshot {
        item_name: "Chilli".to_owned(),
        nutrition: NutritionFacts {
            energy_kcal: Some(Decimal::new(800, 0)),
            protein_g: Some(Decimal::new(40, 0)),
            ..NutritionFacts::default()
        },
        quality: NutritionQuality::Known,
    };
    let per_unit = snapshot.per_unit(&servings(4));
    assert_eq!(per_unit.energy_kcal, Some(Decimal::new(200, 0)));

    let two = snapshot.scaled_to(&servings(4), &servings(2));
    assert_eq!(two.energy_kcal, Some(Decimal::new(400, 0)));
    assert_eq!(two.protein_g, Some(Decimal::new(20, 0)));
    assert_eq!(two.basis, None);
}

#[test]
fn participants_validate_against_the_meal_components() {
    let comp = component(MealPlanComponentId::new(), servings(4));
    let member = HouseholdMemberId::new();

    let ok = vec![NewMealParticipant {
        id: None,
        member_id: member,
        allocations: vec![NewMealParticipantAllocation {
            component_id: comp.id,
            allocated: servings(2),
        }],
    }];
    assert!(validate_participants(&ok, std::slice::from_ref(&comp)).is_ok());

    let duplicate_member = vec![
        NewMealParticipant {
            id: None,
            member_id: member,
            allocations: vec![],
        },
        NewMealParticipant {
            id: None,
            member_id: member,
            allocations: vec![],
        },
    ];
    assert!(validate_participants(&duplicate_member, std::slice::from_ref(&comp)).is_err());

    let wrong_kind = vec![NewMealParticipant {
        id: None,
        member_id: member,
        allocations: vec![NewMealParticipantAllocation {
            component_id: comp.id,
            allocated: grams(100),
        }],
    }];
    assert!(validate_participants(&wrong_kind, std::slice::from_ref(&comp)).is_err());

    let unknown_component = vec![NewMealParticipant {
        id: None,
        member_id: member,
        allocations: vec![NewMealParticipantAllocation {
            component_id: MealPlanComponentId::new(),
            allocated: servings(1),
        }],
    }];
    assert!(validate_participants(&unknown_component, std::slice::from_ref(&comp)).is_err());
}

fn meal_times() -> MealTimes {
    MealTimes {
        breakfast: time::macros::time!(08:00),
        lunch: time::macros::time!(12:30),
        dinner: time::macros::time!(18:00),
    }
}

fn at(date: time::Date, clock: time::Time) -> OffsetDateTime {
    date.with_time(clock).assume_utc()
}

#[test]
fn a_meal_is_assumed_once_its_own_planned_time_has_passed() {
    let day = time::macros::date!(2026 - 09 - 02);

    let before = Assumption::for_occurrence(
        day,
        Some(time::macros::time!(13:00)),
        MealSlot::Lunch,
        at(
            time::macros::date!(2026 - 09 - 02),
            time::macros::time!(12:59),
        ),
        &meal_times(),
        true,
    );
    assert!(!before.assumed);

    let after = Assumption::for_occurrence(
        day,
        Some(time::macros::time!(13:00)),
        MealSlot::Lunch,
        at(
            time::macros::date!(2026 - 09 - 02),
            time::macros::time!(13:01),
        ),
        &meal_times(),
        true,
    );
    assert!(after.assumed);
}

#[test]
fn a_meal_without_a_time_falls_back_to_the_household_slot_time() {
    let day = time::macros::date!(2026 - 09 - 02);

    let before = Assumption::for_occurrence(
        day,
        None,
        MealSlot::Dinner,
        at(
            time::macros::date!(2026 - 09 - 02),
            time::macros::time!(17:30),
        ),
        &meal_times(),
        true,
    );
    assert!(!before.assumed);

    let after = Assumption::for_occurrence(
        day,
        None,
        MealSlot::Dinner,
        at(
            time::macros::date!(2026 - 09 - 02),
            time::macros::time!(18:30),
        ),
        &meal_times(),
        true,
    );
    assert!(after.assumed);
}

#[test]
fn an_untimed_snack_is_only_assumed_once_its_day_is_over() {
    let day = time::macros::date!(2026 - 09 - 02);

    let same_day = Assumption::for_occurrence(
        day,
        None,
        MealSlot::Snacks,
        at(
            time::macros::date!(2026 - 09 - 02),
            time::macros::time!(23:00),
        ),
        &meal_times(),
        true,
    );
    assert!(!same_day.assumed);

    let next_day = Assumption::for_occurrence(
        day,
        None,
        MealSlot::Snacks,
        at(
            time::macros::date!(2026 - 09 - 03),
            time::macros::time!(00:01),
        ),
        &meal_times(),
        true,
    );
    assert!(next_day.assumed);
}

#[test]
fn turning_the_setting_off_stops_anything_being_assumed() {
    let assumption = Assumption::for_occurrence(
        time::macros::date!(2026 - 09 - 01),
        Some(time::macros::time!(08:00)),
        MealSlot::Breakfast,
        at(
            time::macros::date!(2026 - 09 - 02),
            time::macros::time!(12:00),
        ),
        &meal_times(),
        false,
    );
    assert!(!assumption.assumed);
}

#[test]
fn an_unresolved_meal_reads_as_assumed_but_a_resolved_one_does_not() {
    let comp = MealPlanComponentId::new();
    let assumed = Assumption::new(true);

    let pending = participant(vec![allocation(comp, servings(1))]);
    assert_eq!(
        derive_participant_status(&pending, assumed),
        MealPlanStatus::Assumed
    );
    assert_eq!(
        derive_entry_status(std::slice::from_ref(&pending), &[], assumed),
        MealPlanStatus::Assumed
    );

    let ate = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::Eaten,
    )]);
    assert_eq!(
        derive_participant_status(&ate, assumed),
        MealPlanStatus::Eaten
    );

    let declined = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::NotEaten,
    )]);
    assert_eq!(
        derive_participant_status(&declined, assumed),
        MealPlanStatus::NotEaten
    );
}

#[test]
fn a_partly_resolved_meal_stays_partly_resolved_rather_than_assumed() {
    let comp = MealPlanComponentId::new();
    let assumed = Assumption::new(true);

    let ate = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::Eaten,
    )]);
    let pending = participant(vec![allocation(comp, servings(1))]);

    assert_eq!(
        derive_entry_status(&[ate, pending], &[], assumed),
        MealPlanStatus::PartiallyResolved
    );
}

#[test]
fn assumed_counts_as_unresolved_so_it_stays_editable() {
    assert!(MealPlanStatus::Assumed.is_unresolved());
    assert!(MealPlanStatus::Planned.is_unresolved());
    assert!(!MealPlanStatus::Eaten.is_unresolved());
    assert!(!MealPlanStatus::NotEaten.is_unresolved());
    assert!(!MealPlanStatus::PartiallyResolved.is_unresolved());
}
