import { describe, expect, it } from 'vitest';
import { formatWeekRange, startOfWeekIso } from '../diary/date';

describe('meal plan dates', () => {
  it('finds Monday for every day in a week', () => {
    expect(startOfWeekIso('2026-08-24')).toBe('2026-08-24');
    expect(startOfWeekIso('2026-08-30')).toBe('2026-08-24');
  });

  it('formats a week across month boundaries', () => {
    expect(formatWeekRange('2026-08-31')).toBe('31 Aug to 6 Sept 2026');
  });
});
