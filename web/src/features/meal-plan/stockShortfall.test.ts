import { describe, expect, it } from 'vitest';
import type { components } from '../../api/schema';
import { collectStockOutcomes, describeStockOutcome } from './stockShortfall';

type StockOutcome = components['schemas']['StockOutcomeDto'];

function outcome(over: Partial<StockOutcome>): StockOutcome {
  return {
    subject: { kind: 'product', product_id: 'p1' },
    name: 'Chicken Breast',
    wanted: { amount: 300, unit: 'g' },
    deducted: { amount: 300, unit: 'g' },
    shortfall: { state: 'covered' },
    unresolved_release: false,
    ...over,
  };
}

describe('describeStockOutcome', () => {
  it('names the uncovered amount for a definite shortfall', () => {
    const text = describeStockOutcome(
      outcome({
        deducted: { amount: 150, unit: 'g' },
        shortfall: { state: 'short', amount: { amount: 150, unit: 'g' }, confidence: 'exact' },
      }),
    );
    expect(text).toContain('Only 150 g of Chicken Breast');
    expect(text).toContain('150 g is unaccounted for');
  });

  it('hedges when the covering stock was an estimate', () => {
    const text = describeStockOutcome(
      outcome({
        shortfall: { state: 'short', amount: { amount: 100, unit: 'g' }, confidence: 'estimated' },
      }),
    );
    expect(text).toContain('may not have covered 100 g');
    expect(text).toContain('estimate');
  });

  it('says the answer is unknown for an incompatible unit', () => {
    const text = describeStockOutcome(
      outcome({ shortfall: { state: 'indeterminate', amount: { amount: 100, unit: 'g' } } }),
    );
    expect(text).toContain('cannot compare');
  });

  it('flags an unresolved release', () => {
    const text = describeStockOutcome(outcome({ unresolved_release: true }));
    expect(text).toContain('could not be put back');
  });
});

describe('collectStockOutcomes', () => {
  it('flattens and drops empties', () => {
    expect(
      collectStockOutcomes([
        undefined,
        { stock_outcomes: [] },
        { stock_outcomes: [outcome({})] },
      ]),
    ).toHaveLength(1);
  });
});
