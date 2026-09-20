import { describe, expect, it } from 'vitest';
import type { Availability, Unit } from '../../api/client';
import { levelFor } from './stockLevel';

const quantified = (
  onHand: number,
  planned: number,
  unit: Unit,
  confidence: 'exact' | 'estimated' = 'exact',
): Availability => ({
  state: 'quantified',
  confidence,
  on_hand: { amount: onHand, unit },
  planned_demand: { amount: planned, unit },
  unallocated: { amount: onHand - planned, unit },
});

describe('levelFor', () => {
  it('flags a shortfall with the shortfall amount and no fill', () => {
    const level = levelFor(quantified(1200, 1800, 'g'));
    expect(level.figure?.short).toBe(true);
    expect(level.figure?.onHand).toBe('1,200 g');
    expect(level.figure?.free).toBe('600 g short');
    expect(level.figure?.shortAmount).toBe('600 g');
    expect(level.figure?.fillPct).toBe(0);
    expect(level.tier).toBe('amber');
    expect(level.sortRank).toBe(0);
  });

  it('shows an empty gauge when exactly nothing is free', () => {
    const level = levelFor(quantified(650, 650, 'g'));
    expect(level.figure?.short).toBe(false);
    expect(level.figure?.fillPct).toBe(0);
    expect(level.figure?.free).toBe('0 g free');
  });

  it('fills the gauge fully when nothing is planned', () => {
    const level = levelFor(quantified(4, 0, 'pack'));
    expect(level.figure?.fillPct).toBe(100);
    expect(level.figure?.onHand).toBe('4 packs');
    expect(level.figure?.free).toBe('4 free');
  });

  it('marks an estimated on-hand figure with a tilde', () => {
    const level = levelFor(quantified(4050, 2100, 'ml', 'estimated'));
    expect(level.figure?.onHand).toBe('~4,050 ml');
    expect(level.figure?.free).toBe('1,950 ml free');
  });

  it('drops the unit from a countable shortfall beside the on-hand figure but keeps it on its own', () => {
    const level = levelFor(quantified(2, 5, 'pack'));
    expect(level.figure?.onHand).toBe('2 packs');
    expect(level.figure?.free).toBe('3 short');
    expect(level.figure?.shortAmount).toBe('3 packs');
  });

  it('reads nothing on hand against a planned meal as short', () => {
    const level = levelFor(quantified(0, 500, 'g'));
    expect(level.figure?.short).toBe(true);
    expect(level.figure?.onHand).toBe('0 g');
    expect(level.figure?.free).toBe('500 g short');
    expect(level.freeFraction).toBe(-1);
  });

  it('keeps the unit in the free figure for a measured unit', () => {
    const level = levelFor(quantified(1460, 335, 'g'));
    expect(level.figure?.free).toBe('1,125 g free');
  });

  it('reads a product not tracked against stock as not counted', () => {
    const level = levelFor({ state: 'assumed_available' });
    expect(level.statusWord).toBe('Not counted');
    expect(level.figure).toBeNull();
  });

  it('reads an unresolvable availability as not known', () => {
    expect(levelFor({ state: 'unknown' }).statusWord).toBe('Not known');
    expect(levelFor(null).statusWord).toBe('Not known');
    expect(levelFor(undefined).statusWord).toBe('Not known');
  });

  it('reads an absent product as none in stock', () => {
    const level = levelFor({ state: 'absent' });
    expect(level.statusWord).toBe('None in stock');
    expect(level.figure).toBeNull();
  });
});
