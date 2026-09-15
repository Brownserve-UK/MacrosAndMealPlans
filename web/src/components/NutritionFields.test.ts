import { describe, expect, it } from 'vitest';
import {
  derivedBasis,
  draftToNutrition,
  isBasisEditable,
  nutritionToDraft,
  validateNutritionDraft,
  type PackContext,
} from './NutritionFields';

const NO_PACK: PackContext = {};
const MASS_PACK: PackContext = { quantity: { amount: 600, unit: 'g' } };
const VOLUME_PACK: PackContext = { quantity: { amount: 1, unit: 'l' } };
const COUNT_PACK: PackContext = { quantity: { amount: 1, unit: 'item' } };
const per100g = { basisAmount: '100', basisUnit: 'g' } as const;

describe('derived basis', () => {
  it('defaults to per 100g when there is no pack yet', () => {
    expect(derivedBasis(NO_PACK)).toEqual({ amount: 100, unit: 'g' });
    expect(isBasisEditable(NO_PACK)).toBe(true);
  });

  it('defaults to per 100ml for a volume pack', () => {
    expect(derivedBasis(VOLUME_PACK)).toEqual({ amount: 100, unit: 'ml' });
    expect(isBasisEditable(VOLUME_PACK)).toBe(true);
  });

  it('defaults to per item for a counted pack, because a label cannot say per 100 eggs', () => {
    expect(derivedBasis(COUNT_PACK)).toEqual({ amount: 1, unit: 'item' });
    expect(isBasisEditable(COUNT_PACK)).toBe(false);
  });

  it('derives a per-serving quantity from the pack once servings are known', () => {
    const buns: PackContext = { quantity: { amount: 600, unit: 'g' }, servings: 6 };
    expect(derivedBasis(buns)).toEqual({ amount: 100, unit: 'g' });
    expect(isBasisEditable(buns)).toBe(false);
  });

  it('handles a single item divided into servings, like a quarter of a pizza', () => {
    const pizza: PackContext = { quantity: { amount: 1, unit: 'item' }, servings: 4 };
    expect(derivedBasis(pizza)).toEqual({ amount: 0.25, unit: 'item' });
    expect(isBasisEditable(pizza)).toBe(false);
  });
});

describe('nutrition form conversion', () => {
  it('treats a blank box as unknown, not zero', () => {
    const nutrition = draftToNutrition(
      { ...per100g, energy_kcal: '', protein_g: '' },
      MASS_PACK,
    );
    expect(nutrition.energy_kcal).toBeNull();
    expect(nutrition.protein_g).toBeNull();
  });

  it('keeps an explicit zero as a known value', () => {
    expect(draftToNutrition({ ...per100g, fat_g: '0' }, MASS_PACK).fat_g).toBe(0);
  });

  it('round trips a typed amount against the derived unit', () => {
    const draft = nutritionToDraft({ basis: { amount: 30, unit: 'g' }, energy_kcal: 120 }, MASS_PACK);
    expect(draft.basisAmount).toBe('30');
    expect(draft.basisCustom).toBe(false);

    const back = draftToNutrition(draft, MASS_PACK);
    expect(back.basis).toEqual({ amount: 30, unit: 'g' });
  });

  it('always fills in the derived basis once servings are known, regardless of draft input', () => {
    const buns: PackContext = { quantity: { amount: 600, unit: 'g' }, servings: 6 };
    expect(draftToNutrition({ energy_kcal: '250' }, buns).basis).toEqual({ amount: 100, unit: 'g' });
  });

  it('represents the pizza case as a fraction of an item', () => {
    const pizza: PackContext = { quantity: { amount: 1, unit: 'item' }, servings: 4 };
    expect(draftToNutrition({ energy_kcal: '250' }, pizza).basis).toEqual({
      amount: 0.25,
      unit: 'item',
    });
  });

  it('detects a stored basis that does not match the derived one as custom', () => {
    const draft = nutritionToDraft({ basis: { amount: 100, unit: 'kg' } }, MASS_PACK);
    expect(draft.basisCustom).toBe(true);
    expect(draft.basisUnit).toBe('kg');
  });

  it('respects a custom basis even when it happens to equal the pack amount', () => {
    const draft = { basisCustom: true, basisAmount: '2', basisUnit: 'tbsp' as const };
    expect(draftToNutrition(draft, MASS_PACK).basis).toEqual({ amount: 2, unit: 'tbsp' });
  });

  it('clears the basis when the editable amount is not filled in', () => {
    expect(draftToNutrition({ basisAmount: '' }, MASS_PACK).basis).toBeNull();
  });
});

describe('nutrition validation', () => {
  it('accepts a wholly empty form', () => {
    expect(validateNutritionDraft({}, MASS_PACK)).toEqual({});
  });

  it('requires an amount once any value is given, for an editable basis', () => {
    expect(validateNutritionDraft({ energy_kcal: '250', basisAmount: '' }, MASS_PACK).basis).toBeDefined();
  });

  it('never requires anything once the basis is fully derived', () => {
    const buns: PackContext = { quantity: { amount: 600, unit: 'g' }, servings: 6 };
    expect(validateNutritionDraft({ energy_kcal: '250' }, buns)).toEqual({});
  });

  it('rejects a basis of zero', () => {
    const errors = validateNutritionDraft({ basisAmount: '0', fat_g: '1' }, MASS_PACK);
    expect(errors.basis).toBeDefined();
  });

  it('rejects a negative nutrient', () => {
    expect(validateNutritionDraft({ ...per100g, protein_g: '-1' }, MASS_PACK).protein_g).toBe(
      'Must not be negative',
    );
  });

  it('rejects something that is not a number', () => {
    expect(validateNutritionDraft({ ...per100g, fat_g: 'lots' }, MASS_PACK).fat_g).toBe(
      'Must be a number',
    );
  });

  it('allows zero', () => {
    expect(validateNutritionDraft({ ...per100g, fat_g: '0' }, MASS_PACK)).toEqual({});
  });
});
