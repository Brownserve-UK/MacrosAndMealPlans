import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DailyNutritionSummary } from './DailyNutritionSummary';

const nutrition = {
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

describe('DailyNutritionSummary', () => {
  it('shows the macro split and secondary nutrition', () => {
    render(<DailyNutritionSummary nutrition={nutrition} incompleteCount={0} />);

    expect(
      screen.getByRole('img', {
        name: 'Macro split: Protein 70.4%, Carbs 18.3%, Fat 11.3%',
      }),
    ).toBeInTheDocument();
    expect(screen.getByText('Sugars')).toBeInTheDocument();
  });

  it('shows when an item has incomplete nutrition', () => {
    render(<DailyNutritionSummary nutrition={nutrition} incompleteCount={1} />);
    expect(screen.getByText('1 item has incomplete nutrition')).toBeInTheDocument();
  });

  it('gives equal gram values equal shares', () => {
    render(
      <DailyNutritionSummary
        nutrition={{ protein_g: 1, carbohydrate_g: 1, fat_g: 1 }}
        incompleteCount={0}
      />,
    );
    expect(
      screen.getByRole('img', {
        name: 'Macro split: Protein 33.3%, Carbs 33.3%, Fat 33.3%',
      }),
    ).toBeInTheDocument();
  });
});
