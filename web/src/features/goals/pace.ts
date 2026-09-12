import type { Pace, WeightObjective } from '../../api/client';

const RATES: Record<Pace, number> = {
  steady: 0.25,
  standard: 0.5,
  faster: 0.75,
  fastest: 1,
};

export const PACE_OPTIONS: { value: Pace; label: string }[] = [
  { value: 'steady', label: 'Steady' },
  { value: 'standard', label: 'Standard' },
  { value: 'faster', label: 'Faster' },
  { value: 'fastest', label: 'Fastest' },
];

export function paceOptionsFor(objective: WeightObjective) {
  return objective === 'gain' ? PACE_OPTIONS.slice(0, 2) : PACE_OPTIONS;
}

export function paceRate(pace: Pace): number {
  return RATES[pace];
}

export function paceAdjustment(pace: Pace, objective: WeightObjective): string {
  const calories = paceRate(pace) * 1100;
  return `${objective === 'lose' ? '−' : '+'}${calories.toLocaleString('en-GB')} kcal a day`;
}
