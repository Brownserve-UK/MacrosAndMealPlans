import type { Pace, WeightObjective } from '../../api/client';

const RATES: Record<Pace, number> = {
  steady: 0.25,
  standard: 0.5,
  faster: 0.75,
  fastest: 1,
};

export const PACE_OPTIONS: { value: Pace; label: string }[] = [
  { value: 'steady', label: 'Gradual' },
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

export function paceLabel(pace: Pace): string {
  return `${paceRate(pace).toLocaleString('en-GB')} kg/week`;
}

export function pacePoundsLabel(pace: Pace): string {
  return `${(paceRate(pace) * 2.20462).toFixed(1)} lb/week`;
}
