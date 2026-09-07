import type { ShoppingList, ShoppingRequirement, ShoppingTripRow } from '../../api/client';
import type { Listed } from './grouping';

function subjectOf(row: {
  ingredient_id?: string | null;
  product_id?: string | null;
  name: string;
}): string {
  if (row.product_id) return `product:${row.product_id}`;
  if (row.ingredient_id) return `ingredient:${row.ingredient_id}`;
  return `name:${row.name}`;
}

function subjectOfRequirement(requirement: ShoppingRequirement): string {
  const subject = requirement.subject;
  if (subject.kind === 'product') return `product:${subject.product_id}`;
  if (subject.kind === 'ingredient') return `ingredient:${subject.ingredient_id}`;
  return `other:${requirement.name}`;
}

function fromRow(row: ShoppingTripRow, on: string): ShoppingRequirement {
  return {
    subject: row.product_id
      ? { kind: 'product', product_id: row.product_id }
      : { kind: 'ingredient', ingredient_id: row.ingredient_id ?? row.id },
    name: row.name,
    quantity: row.quantity,
    section: row.section ?? 'other',
    certainty: { kind: 'definite' },
    assignment: { kind: 'opportunity', date: on },
    claims: [],
    gaps: [],
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
    live.set(subjectOfRequirement(requirement), requirement);
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
        : fromRow(row, trip.opportunity_date),
    });
  }

  const added = list.requirements.filter(
    (requirement) =>
      !seen.has(subjectOfRequirement(requirement)) &&
      requirement.assignment.kind === 'opportunity',
  );

  return { rows, added };
}
