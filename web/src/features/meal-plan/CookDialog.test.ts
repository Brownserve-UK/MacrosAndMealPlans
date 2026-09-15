import { describe, expect, it } from 'vitest';
import { scaledAmount } from './CookDialog';

describe('what a cook takes out of stock', () => {
  it('scales a measured ingredient to how much was made', () => {
    expect(scaledAmount({ kind: 'measure', value: 600, unit: 'g' }, 6 / 4)).toBe('900 g');
    expect(scaledAmount({ kind: 'measure', value: 300, unit: 'g' }, 1 / 4)).toBe('75 g');
  });

  it('leaves the recipe amounts alone when you make exactly its yield', () => {
    expect(scaledAmount({ kind: 'measure', value: 600, unit: 'g' }, 1)).toBe('600 g');
  });

  it('rounds to one decimal rather than showing a recurring figure', () => {
    expect(scaledAmount({ kind: 'measure', value: 100, unit: 'g' }, 1 / 3)).toBe('33.3 g');
  });

  it('reads units the way a person writes them', () => {
    expect(scaledAmount({ kind: 'measure', value: 2, unit: 'fl_oz' }, 1)).toBe('2 fl oz');
    expect(scaledAmount({ kind: 'packs', value: 2 }, 1)).toBe('2 packs');
    expect(scaledAmount({ kind: 'packs', value: 2 }, 0.5)).toBe('1 pack');
    expect(scaledAmount({ kind: 'servings', value: 4 }, 0.25)).toBe('1 serving');
  });
});
