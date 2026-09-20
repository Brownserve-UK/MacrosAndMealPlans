import type { Availability } from '../../api/client';
import { formatQuantity } from './SpokenFor';

export type Tier = 'green' | 'amber' | 'muted';

const TIER_COLOUR: Record<Tier, string> = {
  green: 'success.main',
  amber: 'warning.main',
  muted: 'text.secondary',
};

const COUNTABLE_UNITS = new Set([
  'item',
  'piece',
  'slice',
  'clove',
  'can',
  'pack',
  'bunch',
  'serving',
]);

export type StockFigure = {
  onHand: string;
  free: string;
  short: boolean;
  shortAmount: string | null;
  fillPct: number;
};

export type StockLevel = {
  tier: Tier;
  colour: string;
  figure: StockFigure | null;
  statusWord: string | null;
  sortRank: number;
  freeFraction: number;
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function amountLabel(amount: number): string {
  return Number.isInteger(amount)
    ? amount.toLocaleString('en-GB')
    : amount.toLocaleString('en-GB', { maximumFractionDigits: 2 });
}

export function levelFor(availability: Availability | null | undefined): StockLevel {
  const muted = (statusWord: string, sortRank: number): StockLevel => ({
    tier: 'muted',
    colour: TIER_COLOUR.muted,
    figure: null,
    statusWord,
    sortRank,
    freeFraction: Number.POSITIVE_INFINITY,
  });

  if (!availability) return muted('Not known', 5);
  switch (availability.state) {
    case 'assumed_available':
      return muted('Not counted', 4);
    case 'unknown':
      return muted('Not known', 5);
    case 'absent':
      return muted('None in stock', 4.5);
    case 'quantified': {
      const onHand = availability.on_hand.amount;
      const free = availability.unallocated.amount;
      const unit = availability.on_hand.unit;
      const estimated = availability.confidence === 'estimated';
      const countable = COUNTABLE_UNITS.has(unit);
      const f = onHand > 0 ? free / onHand : free >= 0 ? 1 : -1;
      const onHandFigure = `${estimated ? '~' : ''}${formatQuantity({ amount: onHand, unit })}`;

      if (free < 0) {
        const shortAmount = formatQuantity({ amount: -free, unit });
        const shortLabel = countable ? amountLabel(-free) : shortAmount;
        return {
          tier: 'amber',
          colour: TIER_COLOUR.amber,
          figure: {
            onHand: onHandFigure,
            free: `${shortLabel} short`,
            short: true,
            shortAmount,
            fillPct: 0,
          },
          statusWord: null,
          sortRank: 0,
          freeFraction: f,
        };
      }

      const freeLabel = countable ? `${amountLabel(free)} free` : `${formatQuantity({ amount: free, unit })} free`;

      return {
        tier: 'green',
        colour: TIER_COLOUR.green,
        figure: {
          onHand: onHandFigure,
          free: freeLabel,
          short: false,
          shortAmount: null,
          fillPct: clamp(f * 100, 0, 100),
        },
        statusWord: null,
        sortRank: 1,
        freeFraction: f,
      };
    }
  }
}
