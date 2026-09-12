import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { WeightPoint } from '../../api/client';
import { WeightChart } from './WeightChart';

function points(...pairs: [string, number][]): WeightPoint[] {
  return pairs.map(([on, weight_kg]) => ({ on, weight_kg }));
}

describe('WeightChart', () => {
  it('says there is not enough data rather than drawing one point', () => {
    render(<WeightChart points={points(['2026-09-01', 80])} display="kilograms" />);

    expect(screen.getByText(/Not enough data/)).toBeInTheDocument();
    expect(screen.queryByRole('img')).not.toBeInTheDocument();
  });

  it('says there is not enough data when there is none at all', () => {
    render(<WeightChart points={[]} display="kilograms" />);

    expect(screen.getByText(/Not enough data/)).toBeInTheDocument();
  });

  it('describes the trend in words for anyone not looking at it', () => {
    render(
      <WeightChart
        points={points(['2026-09-01', 82], ['2026-09-08', 81], ['2026-09-15', 80])}
        display="kilograms"
      />,
    );

    const label = screen.getByRole('img').getAttribute('aria-label') ?? '';
    expect(label).toMatch(/^Weight down from 82 kg on 1 Sept?/);
    expect(label).toMatch(/to 80 kg on 15 Sept?/);
  });

  it('describes the trend in the units the member picked', () => {
    render(
      <WeightChart points={points(['2026-09-01', 82], ['2026-09-15', 80])} display="stones_pounds" />,
    );

    expect(screen.getByRole('img').getAttribute('aria-label')).toContain('st');
  });

  it('draws the goal line and labels it when the goal is in range', () => {
    render(
      <WeightChart
        points={points(['2026-09-01', 82], ['2026-09-15', 80])}
        goalKg={79}
        display="kilograms"
      />,
    );

    expect(screen.getByText('Goal 79 kg')).toBeInTheDocument();
    expect(document.querySelector('line')).not.toBeNull();
  });

  it('leaves the goal line off when the goal is nowhere near the chart', () => {
    render(
      <WeightChart
        points={points(['2026-09-01', 82], ['2026-09-15', 80])}
        goalKg={40}
        display="kilograms"
      />,
    );

    expect(screen.queryByText(/^Goal/)).not.toBeInTheDocument();
    expect(document.querySelector('line')).toBeNull();
  });

  it('survives a run of identical weights without dividing by zero', () => {
    render(
      <WeightChart
        points={points(['2026-09-01', 80], ['2026-09-08', 80], ['2026-09-15', 80])}
        display="kilograms"
      />,
    );

    const polyline = document.querySelector('polyline');
    expect(polyline).not.toBeNull();
    expect(polyline?.getAttribute('points')).not.toContain('NaN');
    expect(screen.getByRole('img').getAttribute('aria-label')).toMatch(
      /^Weight unchanged from 80 kg on 1 Sept? to 80 kg on 15 Sept?$/,
    );
  });

  it('spaces points by real time, so a gap in weighing shows as a gap', () => {
    render(
      <WeightChart
        points={points(['2026-09-01', 82], ['2026-09-02', 81], ['2026-10-01', 80])}
        display="kilograms"
      />,
    );

    const coords = (document.querySelector('polyline')?.getAttribute('points') ?? '')
      .split(' ')
      .map((pair) => Number(pair.split(',')[0]));

    const first = (coords[1] ?? 0) - (coords[0] ?? 0);
    const second = (coords[2] ?? 0) - (coords[1] ?? 0);
    expect(second).toBeGreaterThan(first * 10);
  });
});
