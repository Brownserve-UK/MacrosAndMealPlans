import { describe, expect, it } from 'vitest';
import { formatQuantity } from './SpokenFor';

describe('formatQuantity', () => {
  it('pluralises the units that are words', () => {
    expect(formatQuantity({ amount: 1, unit: 'item' })).toBe('1 item');
    expect(formatQuantity({ amount: 2, unit: 'item' })).toBe('2 items');
    expect(formatQuantity({ amount: 3, unit: 'bunch' })).toBe('3 bunches');
    expect(formatQuantity({ amount: 0, unit: 'slice' })).toBe('0 slices');
  });

  it('leaves the symbols alone', () => {
    expect(formatQuantity({ amount: 1, unit: 'g' })).toBe('1 g');
    expect(formatQuantity({ amount: 500, unit: 'ml' })).toBe('500 ml');
    expect(formatQuantity({ amount: 2, unit: 'fl_oz' })).toBe('2 fl oz');
  });

  it('still groups thousands', () => {
    expect(formatQuantity({ amount: 2000, unit: 'ml' })).toBe('2,000 ml');
  });
});
