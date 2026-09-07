import { describe, expect, it } from 'vitest';
import type { ShoppingList, ShoppingRequirement, ShoppingTrip } from '../../api/client';
import { pinnedList } from './baseline';

function requirement(
  ingredientId: string,
  name: string,
  amount: number,
): ShoppingRequirement {
  return {
    subject: { kind: 'ingredient', ingredient_id: ingredientId },
    name,
    quantity: { amount, unit: 'g' },
    section: 'meat_fish',
    certainty: { kind: 'definite' },
    assignment: { kind: 'opportunity', date: '2026-09-09' },
    claims: [],
    gaps: [],
  };
}

function list(trip: ShoppingTrip | undefined, requirements: ShoppingRequirement[]): ShoppingList {
  return {
    opportunities: [],
    focus: '2026-09-09',
    cadence_configured: true,
    manual: [],
    unplanned: [],
    counts: [],
    requirements,
    trip,
  };
}

function trip(state: ShoppingTrip['state'], rows: ShoppingTrip['rows']): ShoppingTrip {
  return {
    id: 't1',
    opportunity_date: '2026-09-09',
    state,
    started_at: '2026-09-09T09:00:00Z',
    rows,
    revision: 1,
  };
}

describe('pinnedList', () => {
  it('leaves the list live until a trip is started', () => {
    expect(pinnedList(list(undefined, [requirement('chicken', 'Chicken', 300)]))).toBeNull();
    expect(
      pinnedList(list(trip('finished', []), [requirement('chicken', 'Chicken', 300)])),
    ).toBeNull();
  });

  it('keeps the amount the shopper started with when the plan changes underneath', () => {
    const pinned = pinnedList(
      list(
        trip('shopping', [
          { id: 'r1', ingredient_id: 'chicken', name: 'Chicken', quantity: { amount: 300, unit: 'g' }, section: 'meat_fish' },
        ]),
        [requirement('chicken', 'Chicken', 900)],
      ),
    );

    expect(pinned?.rows).toHaveLength(1);
    expect(pinned?.rows[0]?.requirement.quantity).toEqual({ amount: 300, unit: 'g' });
  });

  it('keeps a row the plan has since dropped', () => {
    const pinned = pinnedList(
      list(
        trip('shopping', [
          { id: 'r1', ingredient_id: 'chicken', name: 'Chicken', quantity: { amount: 300, unit: 'g' }, section: 'meat_fish' },
        ]),
        [],
      ),
    );

    expect(pinned?.rows.map((row) => row.requirement.name)).toEqual(['Chicken']);
    expect(pinned?.added).toEqual([]);
  });

  it('does not list something you added by hand twice', () => {
    const withManual: ShoppingList = {
      ...list(
        trip('shopping', [
          { id: 'r1', product_id: 'ketchup', name: 'Tomato Ketchup', section: 'ambient' },
        ]),
        [],
      ),
      manual: [
        { id: 'm1', product_id: 'ketchup', name: 'Tomato Ketchup', section: 'ambient', revision: 1 },
      ],
    };

    expect(pinnedList(withManual)?.rows).toEqual([]);
  });

  it('offers anything new for review rather than slipping it into the list', () => {
    const pinned = pinnedList(
      list(
        trip('shopping', [
          { id: 'r1', ingredient_id: 'chicken', name: 'Chicken', quantity: { amount: 300, unit: 'g' }, section: 'meat_fish' },
        ]),
        [requirement('chicken', 'Chicken', 300), requirement('salmon', 'Salmon', 450)],
      ),
    );

    expect(pinned?.rows.map((row) => row.requirement.name)).toEqual(['Chicken']);
    expect(pinned?.added.map((row) => row.name)).toEqual(['Salmon']);
  });
});
