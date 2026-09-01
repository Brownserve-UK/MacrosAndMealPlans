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
        status: MealPlanStatus::Planned,
        resolved_by: None,
        resolved_at: None,
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
        derive_participant_status(&all_pending),
        MealPlanStatus::Planned
    );

    let one_eaten = participant(vec![
        resolved(allocation(one, servings(1)), ParticipantStatus::Eaten),
        allocation(two, servings(1)),
    ]);
    assert_eq!(
        derive_participant_status(&one_eaten),
        MealPlanStatus::PartiallyResolved
    );

    let eaten_and_declined = participant(vec![
        resolved(allocation(one, servings(1)), ParticipantStatus::Eaten),
        resolved(allocation(two, servings(1)), ParticipantStatus::NotEaten),
    ]);
    assert_eq!(
        derive_participant_status(&eaten_and_declined),
        MealPlanStatus::Eaten
    );

    let all_declined = participant(vec![
        resolved(allocation(one, servings(1)), ParticipantStatus::NotEaten),
        resolved(allocation(two, servings(1)), ParticipantStatus::NotEaten),
    ]);
    assert_eq!(
        derive_participant_status(&all_declined),
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
        derive_component_status(comp, &[ate.clone(), pending]),
        MealPlanStatus::PartiallyResolved
    );

    let declined = participant(vec![resolved(
        allocation(comp, servings(1)),
        ParticipantStatus::NotEaten,
    )]);
    assert_eq!(
        derive_component_status(comp, &[ate, declined]),
        MealPlanStatus::Eaten
    );
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
        derive_entry_status(&[ate.clone(), pending], &[]),
        MealPlanStatus::PartiallyResolved
    );
    assert_eq!(
        derive_entry_status(&[ate, declined], &[]),
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
