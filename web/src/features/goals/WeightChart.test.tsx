import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { WeightPoint } from '../../api/client';
import { WeightChart } from './WeightChart';

const points = (...pairs: [string, number][]): WeightPoint[] => pairs.map(([on, weight_kg]) => ({ on, weight_kg }));

describe('WeightChart', () => {
  it('shows a short empty state', () => { render(<WeightChart points={[]} display="kilograms" />); expect(screen.getByText('Add your first weigh-in to see your trend.')).toBeInTheDocument(); });
  it('describes the visible trend in the selected unit', () => { render(<WeightChart points={points(['2026-09-01', 82], ['2026-09-15', 80])} display="stones_pounds" />); expect(screen.getByRole('img').getAttribute('aria-label')).toMatch(/Weight down.*st.*15 Sept/); });
  it('draws gridlines and a goal line', () => { const { container } = render(<WeightChart points={points(['2026-09-01', 82], ['2026-09-15', 80])} goalKg={75} display="kilograms" />); expect(screen.getByText('Goal 75 kg')).toBeInTheDocument(); expect(container.querySelectorAll('line').length).toBeGreaterThan(3); expect(container.querySelectorAll('circle')).toHaveLength(2); });
  it('lets mouse and touch users inspect and dismiss every point', () => { render(<WeightChart points={points(['2026-09-01', 82], ['2026-09-15', 80])} display="kilograms" />); const point = screen.getByRole('button', { name: '82 kg on 1 Sept' }); expect(screen.getAllByText('82 kg')).toHaveLength(1); fireEvent.click(point); expect(screen.getAllByText('82 kg')).toHaveLength(2); fireEvent.click(point); expect(screen.getAllByText('82 kg')).toHaveLength(1); });
  it('survives identical weights without invalid geometry', () => { const { container } = render(<WeightChart points={points(['2026-09-01', 80], ['2026-09-08', 80])} display="kilograms" />); expect(container.querySelector('path')?.getAttribute('d')).not.toContain('NaN'); });
});
