import { describe, expect, it } from 'vitest';
import { formatAmount } from './format';

describe('formatAmount', () => {
  it('formats a measured amount with its unit', () => {
    expect(formatAmount({ kind: 'measure', value: 150, unit: 'g' })).toBe('150 g');
  });

  it('pluralises servings', () => {
    expect(formatAmount({ kind: 'servings', value: 1 })).toBe('1 serving');
    expect(formatAmount({ kind: 'servings', value: 2 })).toBe('2 servings');
  });

  it('pluralises packs', () => {
    expect(formatAmount({ kind: 'packs', value: 1 })).toBe('1 pack');
    expect(formatAmount({ kind: 'packs', value: 0.5 })).toBe('0.5 packs');
  });
});
