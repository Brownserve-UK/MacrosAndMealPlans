import { describe, expect, it } from 'vitest';
import type { UnitInfo } from '../../api/client';
import {
  amountDraftFrom,
  amountOptionsFor,
  applyOptionKey,
  draftToAmount,
  optionKeyFor,
  unitHint,
  validateAmountDraft,
  type AmountProductInfo,
} from './AmountFields';

const UNITS: UnitInfo[] = [
  { code: 'mg', label: 'milligram', dimension: 'mass', convertible: true },
  { code: 'g', label: 'gram', dimension: 'mass', convertible: true },
  { code: 'kg', label: 'kilogram', dimension: 'mass', convertible: true },
  { code: 'ml', label: 'millilitre', dimension: 'volume', convertible: true },
  { code: 'l', label: 'litre', dimension: 'volume', convertible: true },
  { code: 'item', label: 'item', dimension: 'count', convertible: false },
  { code: 'slice', label: 'slice', dimension: 'count', convertible: false },
  { code: 'pack', label: 'pack', dimension: 'count', convertible: false },
];

const CHICKEN: AmountProductInfo = {
  package_quantity: { amount: 650, unit: 'g' },
  nutrition: { basis: { amount: 100, unit: 'g' } },
};

const MILK: AmountProductInfo = {
  package_quantity: { amount: 1, unit: 'l' },
  nutrition: { basis: { amount: 100, unit: 'ml' } },
};

const EGGS: AmountProductInfo = {
  package_quantity: { amount: 6, unit: 'item' },
  nutrition: { basis: { amount: 1, unit: 'item' } },
};

const PIZZA: AmountProductInfo = {
  package_quantity: { amount: 1, unit: 'item' },
  servings_per_pack: 4,
  nutrition: { basis: { amount: 0.25, unit: 'item' } },
};

const SOLD_BY_THE_PACK: AmountProductInfo = {
  package_quantity: { amount: 1, unit: 'pack' },
  servings_per_pack: 2,
  nutrition: { basis: { amount: 0.5, unit: 'pack' } },
};

const labels = (product: AmountProductInfo | null) =>
  amountOptionsFor(product, UNITS).map((o) => o.label);

describe('amountOptionsFor', () => {
  it('offers only units the product can actually be converted to', () => {
    expect(labels(CHICKEN)).toEqual(['mg', 'g', 'kg', 'pack']);
  });

  it('follows the nutrition basis rather than the pack unit, so a 1 l milk is logged in ml', () => {
    expect(labels(MILK)).toEqual(['ml', 'l', 'pack']);
  });

  it('offers only the exact count unit, because a count unit converts to nothing else', () => {
    expect(labels(EGGS)).toEqual(['item', 'pack']);
    expect(labels(EGGS)).not.toContain('slice');
  });

  it('offers servings only when the product records a serving count', () => {
    expect(labels(PIZZA)).toContain('serving');
    expect(labels(CHICKEN)).not.toContain('serving');
  });

  it('drops a measurement unit whose name a pack or serving has already claimed', () => {
    expect(labels(SOLD_BY_THE_PACK)).toEqual(['serving', 'pack']);
  });

  it('never lists the same label twice, whatever the product', () => {
    for (const product of [CHICKEN, MILK, EGGS, PIZZA, SOLD_BY_THE_PACK, null]) {
      const shown = labels(product);
      expect(shown).toHaveLength(new Set(shown).size);
    }
  });

  it('offers no pack or serving when the product has no pack size', () => {
    const loose: AmountProductInfo = { nutrition: { basis: { amount: 100, unit: 'g' } } };
    expect(labels(loose)).toEqual(['mg', 'g', 'kg']);
  });

  it('falls back to every unit when nothing is known about the product yet', () => {
    expect(labels(null)).toEqual(['mg', 'g', 'kg', 'ml', 'l', 'item', 'slice', 'pack']);
  });
});

describe('option keys', () => {
  it('round-trips a measured unit', () => {
    const draft = amountDraftFrom(CHICKEN);
    expect(optionKeyFor(draft)).toBe('measure:g');
    expect(applyOptionKey(draft, 'measure:kg')).toMatchObject({ kind: 'measure', unit: 'kg' });
  });

  it('round-trips servings and packs without touching the unit', () => {
    const draft = amountDraftFrom(PIZZA);
    const servings = applyOptionKey(draft, 'servings');
    expect(servings.kind).toBe('servings');
    expect(optionKeyFor(servings)).toBe('servings');
    expect(optionKeyFor(applyOptionKey(draft, 'packs'))).toBe('packs');
  });
});

describe('amountDraftFrom', () => {
  it('defaults to the unit the label states its nutrition in', () => {
    expect(amountDraftFrom(MILK).unit).toBe('ml');
    expect(amountDraftFrom(EGGS).unit).toBe('item');
  });

  it('falls back to the pack unit when there is no nutrition basis', () => {
    expect(amountDraftFrom({ package_quantity: { amount: 650, unit: 'g' } }).unit).toBe('g');
  });

  it('falls back to grams when nothing is known', () => {
    expect(amountDraftFrom(null).unit).toBe('g');
  });
});

describe('unitHint', () => {
  it('says how big a pack is when logging by pack', () => {
    expect(unitHint(CHICKEN, { kind: 'packs', value: '1', unit: 'g' })).toBe('1 pack is 650 g');
  });

  it('says how big a serving is, because a serving size is not obvious', () => {
    expect(unitHint(PIZZA, { kind: 'servings', value: '1', unit: 'item' })).toBe(
      '1 serving is 0.25 item',
    );
  });

  it('stays quiet for a plain measured amount, which needs no explaining', () => {
    expect(unitHint(CHICKEN, { kind: 'measure', value: '150', unit: 'g' })).toBeNull();
  });
});

describe('draftToAmount', () => {
  it('builds a measured amount with its unit', () => {
    expect(draftToAmount({ kind: 'measure', value: '150', unit: 'g' })).toEqual({
      kind: 'measure',
      value: 150,
      unit: 'g',
    });
  });

  it('builds a servings amount without a unit', () => {
    expect(draftToAmount({ kind: 'servings', value: '1.5', unit: 'g' })).toEqual({
      kind: 'servings',
      value: 1.5,
    });
  });

  it('returns null for a blank amount rather than zero', () => {
    expect(draftToAmount({ kind: 'measure', value: '  ', unit: 'g' })).toBeNull();
  });

  it('returns null for a value that does not parse as a number', () => {
    expect(draftToAmount({ kind: 'measure', value: 'abc', unit: 'g' })).toBeNull();
  });
});

describe('validateAmountDraft', () => {
  it('requires an amount', () => {
    expect(validateAmountDraft({ kind: 'measure', value: '', unit: 'g' }).amount).toBe('Required');
  });

  it('rejects a non-numeric amount', () => {
    expect(validateAmountDraft({ kind: 'measure', value: 'abc', unit: 'g' }).amount).toBe(
      'Must be a number',
    );
  });

  it('rejects zero and negative amounts', () => {
    expect(validateAmountDraft({ kind: 'measure', value: '0', unit: 'g' }).amount).toBe(
      'Must be more than zero',
    );
    expect(validateAmountDraft({ kind: 'measure', value: '-5', unit: 'g' }).amount).toBe(
      'Must be more than zero',
    );
  });

  it('accepts a positive amount', () => {
    expect(validateAmountDraft({ kind: 'measure', value: '150', unit: 'g' })).toEqual({});
  });
});
