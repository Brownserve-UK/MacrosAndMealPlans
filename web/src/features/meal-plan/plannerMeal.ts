import type { MealPlanEntry, PlannerMeal } from '../../api/client';

export function entryToPlannerMeal(
  entry: MealPlanEntry,
  options: { canRecord: boolean; capabilities: PlannerMeal['capabilities'] },
): PlannerMeal {
  return {
    id: entry.id,
    scope: entry.scope,
    member_id: entry.member_id ?? undefined,
    owner_name: undefined,
    planned_on: entry.planned_on,
    planned_time: entry.planned_time ?? undefined,
    slot: entry.slot,
    status: entry.status,
    foods: entry.components.map((component) => ({
      id: component.id,
      ...(component.item_kind === 'recipe'
        ? { item_kind: 'recipe' as const, recipe_id: component.recipe_id }
        : component.item_kind === 'dish'
          ? { item_kind: 'dish' as const, dish_recipe_id: component.dish_recipe_id }
          : component.item_kind === 'ingredient'
            ? { item_kind: 'ingredient' as const, ingredient_id: component.ingredient_id }
            : component.item_kind === 'prepared_meal'
              ? { item_kind: 'prepared_meal' as const, prepared_meal_id: component.prepared_meal_id }
              : { item_kind: 'product' as const, product_id: component.product_id }),
      item_name: component.item_name,
      amount: component.amount,
      shortage: component.preparation.shortage,
      needs_cooking: component.needs_cooking,
      cooked: component.cooked,
    })),
    people: entry.participants.map((person) => ({
      member_id: person.member_id,
      display_name: person.display_name,
      status: person.status,
      allocations: person.allocations,
      can_record: options.canRecord,
    })),
    guest_groups: entry.guest_groups,
    opted_out: entry.opted_out ?? [],
    can_opt_out: false,
    can_join: false,
    capabilities: options.capabilities,
    revision: entry.revision,
  };
}
