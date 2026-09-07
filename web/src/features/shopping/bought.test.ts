import { describe, expect, it } from 'vitest';
import type { Purchase, ShoppingRequirement } from '../../api/client';
import { boughtSentence, boughtSoFar, convert } from './bought';

function pack(quantity: Purchase['quantity'], id: string): Purchase {
  return {
    id,
    state: 'pending',
    quantity,
    purchased_at: '2026-09-18T10:00:00Z',
    revision: 1,
  };
}

function chicken(purchases: Purchase[], needed = 900): ShoppingRequirement {
  return {
    subject: { kind: 'ingredient', ingredient_id: 'chicken' },
    name: 'Chicken Breast',
    quantity: { amount: needed, unit: 'g' },
    section: 'meat_fish',
    certainty: { kind: 'definite' },
    assignment: { kind: 'opportunity', date: '2026-09-18' },
    claims: [],
    gaps: [],
    purchases,
  };
}

describe('convert', () => {
  it('moves between units of the same dimension', () => {
    expect(convert({ amount: 1.55, unit: 'kg' }, 'g')).toBeCloseTo(1550);
    expect(convert({ amount: 500, unit: 'ml' }, 'l')).toBeCloseTo(0.5);
  });

  it('refuses to guess across dimensions', () => {
    expect(convert({ amount: 500, unit: 'g' }, 'ml')).toBeNull();
    expect(convert({ amount: 2, unit: 'item' }, 'g')).toBeNull();
  });
});

describe('boughtSoFar', () => {
  it('says nothing until something is in the trolley', () => {
    expect(boughtSoFar(chicken([]))).toBeNull();
  });

  it('adds packs up across units and reports the excess', () => {
    const bought = boughtSoFar(
      chicken([pack({ amount: 650, unit: 'g' }, 'a'), pack({ amount: 0.9, unit: 'kg' }, 'b')]),
    );

    expect(bought?.total).toEqual({ amount: 1550, unit: 'g' });
    expect(bought?.difference).toBeCloseTo(650);
    expect(boughtSentence(bought!)).toBe('1.55 kg in the trolley, 650 g more than needed');
  });

  it('reports what is still short', () => {
    const bought = boughtSoFar(chicken([pack({ amount: 600, unit: 'g' }, 'a')]));

    expect(boughtSentence(bought!)).toBe('600 g in the trolley, 300 g short');
  });

  it('never claims a total it could not work out', () => {
    const bought = boughtSoFar(chicken([pack(undefined, 'a'), pack({ amount: 600, unit: 'g' }, 'b')]));

    expect(bought?.incomplete).toBe(true);
    expect(boughtSentence(bought!)).toBe(
      '600 g in the trolley, plus something not written down yet',
    );
  });
});
