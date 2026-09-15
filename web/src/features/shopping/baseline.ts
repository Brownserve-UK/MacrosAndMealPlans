import type {
  Purchase,
  ShoppingList,
  ShoppingRequirement,
  ShoppingTripRow,
} from '../../api/client';
import type { Listed } from './grouping';

function subjectOf(row: {
  ingredient_id?: string | null;
  prepared_meal_id?: string | null;
  product_id?: string | null;
  name: string;
}): string {
  if (row.product_id) return `product:${row.product_id}`;
  if (row.ingredient_id) return `ingredient:${row.ingredient_id}`;
  if (row.prepared_meal_id) return `prepared_meal:${row.prepared_meal_id}`;
  return `name:${row.name}`;
}

function subjectOfRequirement(requirement: ShoppingRequirement): string {
  const subject = requirement.subject;
  if (subject.kind === 'product') return `product:${subject.product_id}`;
  if (subject.kind === 'ingredient') return `ingredient:${subject.ingredient_id}`;
  if (subject.kind === 'prepared_meal') return `prepared_meal:${subject.prepared_meal_id}`;
  return `other:${requirement.name}`;
}

function subjectOfPurchase(purchase: Purchase): string | null {
  if (purchase.product_id) return `product:${purchase.product_id}`;
  if (purchase.ingredient_id) return `ingredient:${purchase.ingredient_id}`;
  if (purchase.prepared_meal_id) return `prepared_meal:${purchase.prepared_meal_id}`;
  return purchase.name ? `name:${purchase.name}` : null;
}

function mergePurchases(...groups: Array<Purchase[] | undefined>): Purchase[] {
  return [...new Map(groups.flatMap((group) => group ?? []).map((item) => [item.id, item])).values()];
}

function fromRow(row: ShoppingTripRow, on: string, purchases: Purchase[]): ShoppingRequirement {
  return {
    subject: row.product_id
      ? { kind: 'product', product_id: row.product_id }
      : row.prepared_meal_id
        ? { kind: 'prepared_meal', prepared_meal_id: row.prepared_meal_id }
      : { kind: 'ingredient', ingredient_id: row.ingredient_id ?? row.id },
    name: row.name,
    quantity: row.quantity,
    section: row.section ?? 'other',
    certainty: { kind: 'definite' },
    assignment: { kind: 'opportunity', date: on },
    claims: [],
    gaps: [],
    purchases,
  };
}

export type Pinned = {
  rows: Listed[];
  added: ShoppingRequirement[];
};

export function pinnedList(list: ShoppingList): Pinned | null {
  const trip = list.trip;
  if (!trip || trip.state !== 'shopping') return null;

  const live = new Map<string, ShoppingRequirement>();
  for (const requirement of list.requirements) {
    const key = subjectOfRequirement(requirement);
    const existing = live.get(key);
    const focused =
      requirement.assignment.kind === 'opportunity' &&
      requirement.assignment.date === trip.opportunity_date;
    const representative = focused || !existing ? requirement : existing;
    live.set(key, {
      ...representative,
      purchases: mergePurchases(existing?.purchases, requirement.purchases),
    });
  }

  const purchases = new Map<string, Purchase[]>();
  for (const purchase of [
    ...list.requirements.flatMap((requirement) => requirement.purchases ?? []),
    ...list.unplanned,
  ]) {
    const key = subjectOfPurchase(purchase);
    if (key) purchases.set(key, mergePurchases(purchases.get(key), [purchase]));
  }

  const byHand = new Set(list.manual.map(subjectOf));

  const rows: Listed[] = [];
  const seen = new Set<string>();
  for (const row of trip.rows) {
    const key = subjectOf(row);
    if (byHand.has(key)) continue;
    const match = seen.has(key) ? undefined : live.get(key);
    seen.add(key);
    rows.push({
      key: row.id,
      requirement: match
        ? {
            ...match,
            quantity: row.quantity ?? match.quantity,
            section: row.section ?? match.section,
          }
        : fromRow(row, trip.opportunity_date, purchases.get(key) ?? []),
    });
  }

  const added = list.requirements.filter(
    (requirement) =>
      !seen.has(subjectOfRequirement(requirement)) &&
      requirement.assignment.kind === 'opportunity',
  );

  return { rows, added };
}
