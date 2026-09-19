import type { components } from '../../api/schema';

type StockOutcome = components['schemas']['StockOutcomeDto'];

function amount(quantity: { amount: number; unit: string }): string {
  return `${quantity.amount} ${quantity.unit}`;
}

export function describeStockOutcome(outcome: StockOutcome): string {
  if (outcome.unresolved_release) {
    return `${outcome.name}: some stock could not be put back automatically because the item's tracking has changed.`;
  }
  const shortfall = outcome.shortfall;
  if (shortfall.state === 'short') {
    if (shortfall.confidence === 'estimated') {
      return `${outcome.name} may not have covered ${amount(shortfall.amount)} — its level is an estimate.`;
    }
    return `Only ${amount(outcome.deducted)} of ${outcome.name} was in stock, so ${amount(shortfall.amount)} is unaccounted for.`;
  }
  if (shortfall.state === 'indeterminate') {
    return `Some ${outcome.name} is measured in a unit we cannot compare, so we cannot tell whether ${amount(shortfall.amount)} was covered.`;
  }
  return `${outcome.name}: stock adjusted.`;
}

export function collectStockOutcomes(
  results: Array<{ stock_outcomes?: StockOutcome[] } | undefined | null>,
): StockOutcome[] {
  return results.flatMap((result) => result?.stock_outcomes ?? []);
}
