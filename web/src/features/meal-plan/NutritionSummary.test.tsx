import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { MealPlanSummary as Summary } from '../../api/client';
import { DayWeekNutrition, type ScopeSummary } from './NutritionSummary';

function summary(
  nutrition: Summary['nutrition'],
  unknownCount = 0,
  partialCount = 0,
): Summary {
  return { nutrition, unknown_count: unknownCount, partial_count: partialCount };
}

function scopeFrom(
  nutrition: Summary['nutrition'],
  unknownCount = 0,
  partialCount = 0,
): ScopeSummary {
  return {
    actual: summary(nutrition),
    remaining: summary(nutrition),
    projected: summary(nutrition, unknownCount, partialCount),
  };
}

const detailed = {
  energy_kcal: 1250,
  protein_g: 40.5,
  carbohydrate_g: 10.5,
  fat_g: 6.5,
  sugar_g: 4.5,
  saturated_fat_g: 2.5,
  fibre_g: 8.5,
  salt_g: 2.5,
  cholesterol_mg: 2.5,
};

const dayScope = scopeFrom(detailed);
const weekScope = scopeFrom({ protein_g: 1, carbohydrate_g: 1, fat_g: 1 });

describe('DayWeekNutrition', () => {
  it('shows the day scope by default with a scope toggle', () => {
    render(<DayWeekNutrition day={dayScope} week={weekScope} />);

    expect(screen.getByRole('button', { name: 'Day' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Week' })).toBeInTheDocument();
    expect(
      screen.getByRole('img', { name: 'Day macro split: Protein 70.4%, Carbs 18.3%, Fat 11.3%' }),
    ).toBeInTheDocument();
    expect(screen.getByText('Sugars')).toBeInTheDocument();
  });

  it('switches to the week scope when toggled', () => {
    render(<DayWeekNutrition day={dayScope} week={weekScope} />);

    fireEvent.click(screen.getByRole('button', { name: 'Week' }));

    expect(
      screen.getByRole('img', { name: 'Week macro split: Protein 33.3%, Carbs 33.3%, Fat 33.3%' }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('img', { name: /Day macro split/ }),
    ).not.toBeInTheDocument();
  });

  it('flags incomplete nutrition for the shown scope', () => {
    render(<DayWeekNutrition day={scopeFrom(detailed, 1, 1)} week={weekScope} />);
    expect(screen.getByText('2 meals have incomplete nutrition')).toBeInTheDocument();
  });
});
