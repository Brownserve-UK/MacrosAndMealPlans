import type { Availability } from '../../api/client';
import { displayUnit } from '../../components/UnitSelect';

export const AMBER = '#D9A441';
export const ORANGE = '#E06C24';

export type Tier = 'green' | 'amber' | 'orange' | 'red' | 'muted';

const TIER_COLOUR: Record<Tier, string> = {
  green: 'success.main',
  amber: AMBER,
  orange: ORANGE,
  red: 'error.main',
  muted: 'text.secondary',
};

export type StockLevel = {
  tier: Tier;
  colour: string;
  fillPct: number;
  solidRed: boolean;
  figure: { needed: string; available: string } | null;
  detailLine: string | null;
  statusWord: string | null;
  sortRank: number;
  freeFraction: number;
};

function quantity(amount: number, unit: string): string {
  const formatted = Number.isInteger(amount)
    ? amount.toLocaleString('en-GB')
    : amount.toLocaleString('en-GB', { maximumFractionDigits: 2 });
  return `${formatted} ${displayUnit(unit)}`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export function levelFor(availability: Availability | null | undefined): StockLevel {
  const muted = (statusWord: string, sortRank: number): StockLevel => ({
    tier: 'muted',
    colour: TIER_COLOUR.muted,
    fillPct: 0,
    solidRed: false,
    figure: null,
    detailLine: null,
    statusWord,
    sortRank,
    freeFraction: Number.POSITIVE_INFINITY,
  });

  if (!availability) return muted('Not known', 5);
  switch (availability.state) {
    case 'assumed_available':
      return muted('Assumed available', 4);
    case 'unknown':
      return muted('Not known', 5);
    case 'absent':
      return muted('None in stock', 4.5);
    case 'quantified': {
      const available = availability.on_hand.amount;
      const needed = availability.planned_demand.amount;
      const free = availability.unallocated.amount;
      const unit = availability.on_hand.unit;
      const estimated = availability.confidence === 'estimated';
      const f = available > 0 ? free / available : free >= 0 ? 1 : -1;

      const figure = {
        needed: quantity(needed, unit),
        available: `${estimated ? '~' : ''}${quantity(available, unit)}`,
      };
      const detailLine =
        needed > 0
          ? `${quantity(available, unit)} on hand · ${quantity(needed, unit)} planned`
          : `${quantity(available, unit)} on hand`;

      if (free < 0) {
        return {
          tier: 'red',
          colour: TIER_COLOUR.red,
          fillPct: 100,
          solidRed: true,
          figure,
          detailLine,
          statusWord: null,
          sortRank: 0,
          freeFraction: f,
        };
      }

      const tier: Tier = f > 0.5 ? 'green' : f > 0.25 ? 'amber' : 'orange';
      const sortRank = tier === 'green' ? 3 : tier === 'amber' ? 2 : 1;
      return {
        tier,
        colour: TIER_COLOUR[tier],
        fillPct: clamp(f * 100, 0, 100),
        solidRed: false,
        figure,
        detailLine,
        statusWord: null,
        sortRank,
        freeFraction: f,
      };
    }
  }
}
