import type { WeightDisplay } from '../../api/client';

const LB_PER_KG = 2.2046226218;
const LB_PER_STONE = 14;

export type WeightUnit = 'kg' | 'lb';

export type WeightInput = { primary: string; secondary: string };

export const EMPTY_INPUT: WeightInput = { primary: '', secondary: '' };

export function kgToLb(kg: number): number {
  return kg * LB_PER_KG;
}

export function lbToKg(lb: number): number {
  return lb / LB_PER_KG;
}

export function unitFor(display: WeightDisplay): WeightUnit {
  return display === 'kilograms' ? 'kg' : 'lb';
}

function round(value: number, places: number): number {
  const factor = 10 ** places;
  return Math.round(value * factor) / factor;
}

function trim(value: number, places: number): string {
  return String(round(value, places));
}

export function formatWeight(kg: number, display: WeightDisplay): string {
  switch (display) {
    case 'pounds':
      return `${trim(kgToLb(kg), 1)} lb`;
    case 'stones_pounds': {
      const totalLb = kgToLb(kg);
      const stones = Math.floor(totalLb / LB_PER_STONE);
      const pounds = round(totalLb - stones * LB_PER_STONE, 1);
      if (pounds >= LB_PER_STONE) return `${stones + 1} st 0 lb`;
      return `${stones} st ${trim(pounds, 1)} lb`;
    }
    default:
      return `${trim(kg, 1)} kg`;
  }
}

export function formatWeightChange(deltaKg: number, display: WeightDisplay): string {
  const rounded = round(deltaKg, 3);
  if (rounded === 0) return `No change`;
  const sign = rounded > 0 ? '+' : '−';
  return `${sign}${formatWeight(Math.abs(rounded), display)}`;
}

export function formatRate(kgPerWeek: number, display: WeightDisplay): string {
  return display === 'kilograms'
    ? `${trim(kgPerWeek, 2)} kg a week`
    : `${trim(kgToLb(kgPerWeek), 1)} lb a week`;
}

export function toInput(kg: number | null | undefined, display: WeightDisplay): WeightInput {
  if (kg == null) return EMPTY_INPUT;
  switch (display) {
    case 'pounds':
      return { primary: trim(kgToLb(kg), 1), secondary: '' };
    case 'stones_pounds': {
      const totalLb = kgToLb(kg);
      const stones = Math.floor(totalLb / LB_PER_STONE);
      const pounds = round(totalLb - stones * LB_PER_STONE, 1);
      if (pounds >= LB_PER_STONE) return { primary: String(stones + 1), secondary: '0' };
      return { primary: String(stones), secondary: trim(pounds, 1) };
    }
    default:
      return { primary: trim(kg, 1), secondary: '' };
  }
}

export function parseWeightInput(
  input: WeightInput,
  display: WeightDisplay,
): { amount: number; unit: WeightUnit } | null {
  const primary = input.primary.trim();
  if (primary === '') return null;
  const first = Number(primary);
  if (!Number.isFinite(first) || first < 0) return null;

  if (display !== 'stones_pounds') {
    if (first <= 0) return null;
    return { amount: round(first, 3), unit: unitFor(display) };
  }

  const secondary = input.secondary.trim();
  const pounds = secondary === '' ? 0 : Number(secondary);
  if (!Number.isFinite(pounds) || pounds < 0 || pounds >= LB_PER_STONE) return null;

  const total = first * LB_PER_STONE + pounds;
  if (total <= 0) return null;
  return { amount: round(total, 3), unit: 'lb' };
}

export function parseWeightInputKg(input: WeightInput, display: WeightDisplay): number | null {
  const parsed = parseWeightInput(input, display);
  if (!parsed) return null;
  return parsed.unit === 'kg' ? parsed.amount : lbToKg(parsed.amount);
}

export function parseRateInput(
  raw: string,
  display: WeightDisplay,
): { amount: number; unit: WeightUnit } | null {
  const trimmed = raw.trim();
  if (trimmed === '') return null;
  const value = Number(trimmed);
  if (!Number.isFinite(value) || value <= 0) return null;
  return { amount: round(value, 3), unit: unitFor(display) };
}

export function rateToInput(kgPerWeek: number | null | undefined, display: WeightDisplay): string {
  if (kgPerWeek == null) return '';
  return display === 'kilograms' ? trim(kgPerWeek, 2) : trim(kgToLb(kgPerWeek), 1);
}

export function unitLabel(display: WeightDisplay): string {
  return display === 'kilograms' ? 'kg' : 'lb';
}
