use super::*;
use crate::domain::{MealItemRef, NutritionFacts, ProductId, Quantity, RecipeId, Unit};
use rust_decimal::Decimal;
use std::str::FromStr;
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
        name: None,
        note: None,
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
        note: None,
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
        cooking_servings: None,
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
fn a_meal_needs_a_component_a_name_or_an_ad_hoc_kind() {
    assert!(validate_group_shape(None, None, &[]).is_err());
    assert!(validate_group_shape(Some("   "), None, &[]).is_err());
    assert!(validate_group_shape(Some("Pizza"), None, &[]).is_ok());
    assert!(validate_group_shape(None, Some(AdHocKind::Takeaway), &[]).is_ok());

    let food = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: servings(1),
        cooking_servings: None,
    }];
    assert!(validate_group_shape(None, None, &food).is_ok());
    assert!(validate_group_shape(None, Some(AdHocKind::EatingOut), &food).is_err());
}

#[test]
fn ad_hoc_kinds_round_trip() {
    for kind in AdHocKind::ALL {
        assert_eq!(AdHocKind::from_str(kind.code()).unwrap(), kind);
    }
}

#[test]
fn a_component_needs_a_positive_amount() {
    let components = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: ConsumedAmount::Servings(Decimal::ZERO),
        cooking_servings: None,
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
        cooking_servings: None,
    }];
    assert!(validate_components(&recipe_component).is_err());

    let product_component = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: grams,
        cooking_servings: None,
    }];
    assert!(validate_components(&product_component).is_ok());
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
        note: None,
        allocations: vec![NewMealParticipantAllocation {
            component_id: comp.id,
            allocated: servings(2),
        }],
    }];
    assert!(validate_participants(&ok, std::slice::from_ref(&comp)).is_ok());

    let duplicate_member = vec![
        NewMealParticipant::member(member),
        NewMealParticipant::member(member),
    ];
    assert!(validate_participants(&duplicate_member, std::slice::from_ref(&comp)).is_err());

    let wrong_kind = vec![NewMealParticipant {
        id: None,
        member_id: member,
        note: None,
        allocations: vec![NewMealParticipantAllocation {
            component_id: comp.id,
            allocated: grams(100),
        }],
    }];
    assert!(validate_participants(&wrong_kind, std::slice::from_ref(&comp)).is_err());

    let unknown_component = vec![NewMealParticipant {
        id: None,
        member_id: member,
        note: None,
        allocations: vec![NewMealParticipantAllocation {
            component_id: MealPlanComponentId::new(),
            allocated: servings(1),
        }],
    }];
    assert!(validate_participants(&unknown_component, std::slice::from_ref(&comp)).is_err());
}

fn group(everyone: bool, members: &[HouseholdMemberId]) -> MealPlanEntry {
    let comp = component(MealPlanComponentId::new(), servings(4));
    MealPlanEntry {
        id: MealPlanEntryId::new(),
        occasion_id: MealOccasionId::new(),
        planned_on: time::macros::date!(2026 - 09 - 15),
        planned_time: None,
        slot: MealSlot::Dinner,
        label: None,
        ad_hoc: None,
        components: vec![comp],
        everyone,
        participants: members
            .iter()
            .map(|member_id| MealParticipant {
                id: MealParticipantId::new(),
                member_id: *member_id,
                note: None,
                allocations: Vec::new(),
                revision: Revision::INITIAL,
                created_at: OffsetDateTime::UNIX_EPOCH,
                updated_at: OffsetDateTime::UNIX_EPOCH,
            })
            .collect(),
        guest_groups: Vec::new(),
        created_by: UserId::new(),
        updated_by: UserId::new(),
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn occasion(groups: Vec<MealPlanEntry>, absent: &[HouseholdMemberId]) -> MealOccasion {
    MealOccasion {
        id: groups
            .first()
            .map(|group| group.occasion_id)
            .unwrap_or_default(),
        planned_on: time::macros::date!(2026 - 09 - 15),
        slot: MealSlot::Dinner,
        planned_time: None,
        note: None,
        groups,
        absences: absent
            .iter()
            .map(|member_id| MealAbsence {
                member_id: *member_id,
                created_by: UserId::new(),
                created_at: OffsetDateTime::UNIX_EPOCH,
            })
            .collect(),
        created_by: UserId::new(),
        updated_by: UserId::new(),
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

#[test]
fn everyone_means_every_active_member_not_seated_elsewhere() {
    let steve = HouseholdMemberId::new();
    let sarah = HouseholdMemberId::new();
    let emily = HouseholdMemberId::new();
    let jack = HouseholdMemberId::new();
    let curry = group(true, &[]);
    let leftovers = group(false, &[sarah]);
    let table = occasion(vec![curry.clone(), leftovers], &[jack]);

    let diners = diners_for(&curry, &table, &[steve, sarah, emily, jack]);
    assert_eq!(diners, vec![steve, emily]);

    assert_eq!(
        table.attendance_of(sarah),
        MealAttendance::Eating {
            group_id: table.groups[1].id,
            note: None
        }
    );
    assert_eq!(table.attendance_of(jack), MealAttendance::Elsewhere);
    assert_eq!(
        table.attendance_of(emily),
        MealAttendance::Eating {
            group_id: curry.id,
            note: None
        }
    );
}

#[test]
fn an_explicit_group_seats_exactly_its_members() {
    let steve = HouseholdMemberId::new();
    let sarah = HouseholdMemberId::new();
    let only_sarah = group(false, &[sarah]);
    let table = occasion(vec![only_sarah.clone()], &[]);

    assert_eq!(
        diners_for(&only_sarah, &table, &[steve, sarah]),
        vec![sarah]
    );
    assert_eq!(table.attendance_of(steve), MealAttendance::Unaccounted);
}

#[test]
fn materialising_seats_the_diners_and_drops_planned_rows_for_people_who_left() {
    let steve = HouseholdMemberId::new();
    let sarah = HouseholdMemberId::new();
    let mut curry = group(true, &[sarah]);
    curry.participants[0].allocations = vec![allocation(curry.components[0].id, servings(1))];

    materialise_participants(&mut curry, &[steve], OffsetDateTime::UNIX_EPOCH);

    let seated: Vec<_> = curry
        .participants
        .iter()
        .map(|participant| participant.member_id)
        .collect();
    assert_eq!(seated, vec![steve]);
    assert_eq!(curry.serves(), 1);
    assert_eq!(curry.participants[0].allocations[0].allocated, servings(1));

    let mut eaten = group(true, &[sarah]);
    eaten.participants[0].allocations = vec![resolved(
        allocation(eaten.components[0].id, servings(1)),
        ParticipantStatus::Eaten,
    )];
    materialise_participants(&mut eaten, &[steve], OffsetDateTime::UNIX_EPOCH);
    assert_eq!(
        eaten.participants.len(),
        2,
        "someone who already ate keeps their row"
    );
}

#[test]
fn serves_counts_diners_and_guests_and_cooking_can_override_it() {
    let steve = HouseholdMemberId::new();
    let sarah = HouseholdMemberId::new();
    let mut roast = group(false, &[steve, sarah]);
    roast.guest_groups = vec![guest_group(Vec::new())];
    roast.guest_groups[0].count = 2;
    assert_eq!(roast.guest_count(), 2);
    assert_eq!(roast.serves(), 4);
    assert_eq!(
        roast.components[0].effective_cooking_servings(roast.serves()),
        4
    );

    roast.components[0].cooking_servings = Some(6);
    assert_eq!(
        roast.components[0].effective_cooking_servings(roast.serves()),
        6
    );
}

#[test]
fn a_group_is_named_by_its_label_kind_or_its_food() {
    let mut pizza = group(true, &[]);
    assert_eq!(
        pizza.display_name(|| vec!["Margherita".to_owned()]),
        "Margherita"
    );
    pizza.label = Some("Pizza night".to_owned());
    assert_eq!(
        pizza.display_name(|| vec!["Margherita".to_owned()]),
        "Pizza night"
    );

    let mut out = group(true, &[]);
    out.components.clear();
    out.ad_hoc = Some(AdHocKind::FendForYourself);
    assert_eq!(out.display_name(Vec::new), "Fend for yourself");
    assert!(!out.is_cooked());
    assert!(!out.is_leftovers());

    let mut leftovers = group(true, &[]);
    leftovers.components[0].item = MealItemRef::dish(RecipeId::new());
    assert!(leftovers.is_leftovers());
    assert!(!leftovers.is_cooked());
}

#[test]
fn a_meal_s_derived_name_joins_its_food_by_count() {
    let one = group(true, &[]);
    assert_eq!(one.display_name(|| vec!["Salmon".to_owned()]), "Salmon");

    let two = group(true, &[]);
    assert_eq!(
        two.display_name(|| vec!["Salmon".to_owned(), "Potatoes".to_owned()]),
        "Salmon & Potatoes"
    );

    let three = group(true, &[]);
    assert_eq!(
        three.display_name(|| vec![
            "Salmon".to_owned(),
            "Potatoes".to_owned(),
            "Broccoli".to_owned(),
        ]),
        "Salmon, Potatoes & Broccoli"
    );

    let four = group(true, &[]);
    assert_eq!(
        four.display_name(|| vec![
            "Chilli".to_owned(),
            "Rice".to_owned(),
            "Garlic bread".to_owned(),
            "Sour cream".to_owned(),
        ]),
        "Chilli, Rice +2"
    );

    let mut labelled = group(true, &[]);
    labelled.label = Some("Fish finger sandwich".to_owned());
    assert_eq!(
        labelled.display_name(|| vec!["Fish fingers".to_owned(), "White bread".to_owned()]),
        "Fish finger sandwich"
    );
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
