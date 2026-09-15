import type { MealSlot, ShoppingList } from '../../api/client';

export type UnreachableMeal = {
  entryId: string;
  plannedOn: string;
  slot: MealSlot;
  recipeName: string | null;
  names: string[];
};

export function unreachableMeals(list: ShoppingList | undefined): UnreachableMeal[] {
  const byEntry = new Map<string, UnreachableMeal>();
  if (!list) return [];

  for (const requirement of list.requirements) {
    if (requirement.assignment.kind !== 'needs_earlier_opportunity') continue;
    for (const claim of requirement.claims) {
      const held = byEntry.get(claim.entry_id) ?? {
        entryId: claim.entry_id,
        plannedOn: claim.planned_on,
        slot: claim.slot,
        recipeName: claim.recipe_name ?? null,
        names: [],
      };
      if (!held.names.includes(requirement.name)) held.names.push(requirement.name);
      byEntry.set(claim.entry_id, held);
    }
  }

  return [...byEntry.values()].sort((a, b) => a.plannedOn.localeCompare(b.plannedOn));
}

export function cannotBuyInTime(names: string[] | undefined): string | null {
  if (!names || names.length === 0) return null;
  const listed =
    names.length === 1
      ? names[0]
      : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
  return `No shop in time for ${listed}`;
}
