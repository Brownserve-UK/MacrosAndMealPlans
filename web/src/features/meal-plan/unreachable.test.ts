import { describe, expect, it } from 'vitest';
import type { ShoppingList } from '../../api/client';
import { cannotBuyInTime, unreachableMeals } from './unreachable';

function claim(entryId: string) {
  return {
    subject: { kind: 'ingredient' as const, ingredient_id: 'i1' },
    quantity: { amount: 1, unit: 'g' as const },
    entry_id: entryId,
    group_name: 'Sausage casserole',
    planned_on: '2026-09-08',
    slot: 'dinner' as const,
    assumed: false,
  };
}

function list(requirements: ShoppingList['requirements']): ShoppingList {
  return {
    opportunities: [],
    requirements,
    manual: [],
    unplanned: [],
    counts: [],
    cadence_configured: true,
    unfinished: [],
    section_order: ['fresh_produce', 'meat_fish', 'dairy', 'bakery', 'frozen', 'ambient', 'drinks', 'household', 'other'],
  };
}

describe('unreachableMeals', () => {
  it('names what a meal cannot get in time', () => {
    const held = unreachableMeals(
      list([
        {
          subject: { kind: 'ingredient', ingredient_id: 'i1' },
          name: 'Sausages',
          section: 'meat_fish',
          certainty: { kind: 'definite' },
          assignment: { kind: 'needs_earlier_opportunity' },
          claims: [claim('meal-1')],
        },
      ]),
    );

    expect(held).toHaveLength(1);
    expect(held[0]).toMatchObject({
      entryId: 'meal-1',
      plannedOn: '2026-09-08',
      slot: 'dinner',
      names: ['Sausages'],
    });
  });

  it('ignores anything a shop can still reach', () => {
    const held = unreachableMeals(
      list([
        {
          subject: { kind: 'ingredient', ingredient_id: 'i1' },
          name: 'Milk',
          section: 'dairy',
          certainty: { kind: 'definite' },
          assignment: { kind: 'opportunity', date: '2026-09-12' },
          claims: [claim('meal-2')],
        },
      ]),
    );

    expect(held).toEqual([]);
  });

  it('gathers several shortfalls onto the one meal', () => {
    const held = unreachableMeals(
      list([
        {
          subject: { kind: 'ingredient', ingredient_id: 'i1' },
          name: 'Sausages',
          section: 'meat_fish',
          certainty: { kind: 'definite' },
          assignment: { kind: 'needs_earlier_opportunity' },
          claims: [claim('meal-1')],
        },
        {
          subject: { kind: 'ingredient', ingredient_id: 'i2' },
          name: 'Onions',
          section: 'fresh_produce',
          certainty: { kind: 'definite' },
          assignment: { kind: 'needs_earlier_opportunity' },
          claims: [claim('meal-1')],
        },
      ]),
    );

    expect(held).toHaveLength(1);
    expect(held[0]?.names).toEqual(['Sausages', 'Onions']);
  });

  it('says nothing when there is nothing to say', () => {
    expect(cannotBuyInTime(undefined)).toBeNull();
    expect(cannotBuyInTime([])).toBeNull();
  });

  it('reads as a sentence', () => {
    expect(cannotBuyInTime(['Sausages'])).toBe('No shop in time for Sausages');
    expect(cannotBuyInTime(['Sausages', 'Onions'])).toBe('No shop in time for Sausages and Onions');
    expect(cannotBuyInTime(['A', 'B', 'C'])).toBe('No shop in time for A, B and C');
  });
});
