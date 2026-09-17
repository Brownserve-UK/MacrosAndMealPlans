import type { MealSlot, ShoppingRequirement } from '../../api/client';
import type { components } from '../../api/schema';

export type MealNeed = { name: string; planned_on: string; slot: MealSlot };

// TODO: swap to generated types
type RequirementWithMeals = ShoppingRequirement & { for_meals?: MealNeed[] };

// TODO: swap to generated types
type CountWithPlanned = components['schemas']['ShopCountDto'] & { planned_count?: number };

export function mealsFor(requirement: ShoppingRequirement): MealNeed[] {
  return (requirement as RequirementWithMeals).for_meals ?? [];
}

export function plannedCountOf(count: components['schemas']['ShopCountDto']): number | null {
  return (count as CountWithPlanned).planned_count ?? null;
}

export function mealsCaption(meals: MealNeed[]): string | null {
  const [first, ...rest] = meals;
  if (!first) return null;
  if (rest.length === 0) return first.name;
  return `${first.name} and ${rest.length} more`;
}

export function plannedCaption(planned: number): string {
  if (planned === 0) return 'nothing planned yet';
  if (planned === 1) return '1 for a planned meal';
  return `${planned} for planned meals`;
}
