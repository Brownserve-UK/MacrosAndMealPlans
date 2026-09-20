import { describe, expect, it } from 'vitest';
import {
  addDays,
  combineDateTime,
  extractTime,
  formatDayLabel,
  formatFullDate,
  formatWeekRange,
  startOfWeekIso,
  toIsoDate,
  todayIso,
} from './date';

describe('toIsoDate', () => {
  it('formats a date as YYYY-MM-DD with zero-padding', () => {
    expect(toIsoDate(new Date(2026, 0, 5))).toBe('2026-01-05');
  });
});

describe('addDays', () => {
  it('adds days within a month', () => {
    expect(addDays('2026-08-22', 1)).toBe('2026-08-23');
  });

  it('subtracts days across a month boundary', () => {
    expect(addDays('2026-08-01', -1)).toBe('2026-07-31');
  });

  it('carries across a year boundary', () => {
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01');
  });
});

describe('formatDayLabel', () => {
  it('labels today, yesterday and tomorrow specially', () => {
    const today = todayIso();
    expect(formatDayLabel(today)).toBe('Today');
    expect(formatDayLabel(addDays(today, -1))).toBe('Yesterday');
    expect(formatDayLabel(addDays(today, 1))).toBe('Tomorrow');
  });

  it('falls back to a short weekday and date for anything else', () => {
    const farAway = addDays(todayIso(), 30);
    const label = formatDayLabel(farAway);
    expect(label).not.toBe('Today');
    expect(label).not.toBe('Yesterday');
    expect(label).not.toBe('Tomorrow');
    expect(label.length).toBeGreaterThan(0);
  });
});

describe('formatFullDate', () => {
  it('formats a full date', () => {
    expect(formatFullDate('2026-08-23')).toBe('Sunday, 23 August 2026');
  });
});

describe('combineDateTime and extractTime', () => {
  it('round-trips a time of day through an instant', () => {
    const instant = combineDateTime('2026-08-22', '14:30');
    expect(extractTime(instant)).toBe('14:30');
  });

  it('presents an instant in the selected local timezone', () => {
    expect(extractTime('2026-08-25T17:30:00Z', 'Europe/London')).toBe('18:30');
  });
});

describe('meal plan dates', () => {
  it('finds Monday for every day in a week', () => {
    expect(startOfWeekIso('2026-08-24')).toBe('2026-08-24');
    expect(startOfWeekIso('2026-08-30')).toBe('2026-08-24');
  });

  it('formats a week across month boundaries', () => {
    expect(formatWeekRange('2026-08-31')).toBe('31 Aug to 6 Sept 2026');
  });
});
