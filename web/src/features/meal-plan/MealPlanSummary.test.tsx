import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { MealPlanSummary as Summary } from '../../api/client';
import { MealPlanSummary } from './MealPlanSummary';

function summary(energy: number, unknownCount = 0, partialCount = 0): Summary {
  return {
    nutrition: {
      energy_kcal: energy,
      protein_g: 120,
      carbohydrate_g: 250,
      fat_g: 70,
    },
    unknown_count: unknownCount,
    partial_count: partialCount,
  };
}

describe('MealPlanSummary', () => {
  it('keeps the projected total as the main figure', () => {
    render(
      <MealPlanSummary
        actual={summary(800)}
        remaining={summary(1600)}
        projected={summary(2400)}
      />,
    );

    expect(screen.getByText('Projected energy')).toBeInTheDocument();
    expect(screen.getByText('2,400', { exact: false })).toBeInTheDocument();
    expect(screen.getByText('Already eaten')).toBeInTheDocument();
    expect(screen.getByText('Still planned')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /Projected macro split/ })).toBeInTheDocument();
  });

  it('only shows an incomplete nutrition warning when needed', () => {
    const { rerender } = render(
      <MealPlanSummary actual={summary(0)} remaining={summary(0)} projected={summary(0)} />,
    );

    expect(screen.queryByText(/incomplete nutrition/)).not.toBeInTheDocument();

    rerender(
      <MealPlanSummary
        actual={summary(0)}
        remaining={summary(0)}
        projected={summary(0, 1, 1)}
      />,
    );

    expect(screen.getByText('2 meals have incomplete nutrition')).toBeInTheDocument();
  });
});
