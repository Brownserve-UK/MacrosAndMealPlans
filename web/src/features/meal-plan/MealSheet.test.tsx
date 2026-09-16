import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { PlannerMeal } from '../../api/client';
import { MealSheet } from './MealSheet';

function meal(overrides: Partial<PlannerMeal> = {}): PlannerMeal {
  return {
    id: 'meal-1',
    mine: true,
    scope: 'household',
    member_id: null,
    planned_on: '2026-09-16',
    planned_time: '18:00',
    slot: 'dinner',
    status: 'planned',
    foods: [
      {
        id: 'food-1',
        item_kind: 'recipe',
        recipe_id: 'recipe-1',
        item_name: 'Chicken and rice',
        amount: { kind: 'servings', value: 3 },
        shortage: true,
        needs_cooking: true,
      },
      {
        id: 'food-2',
        item_kind: 'product',
        product_id: 'product-1',
        item_name: 'Yoghurt',
        amount: { kind: 'measure', value: 150, unit: 'g' },
        shortage: false,
        needs_cooking: false,
      },
    ],
    people: [
      {
        member_id: 'alex',
        display_name: 'Alex Brown',
        status: 'planned',
        can_record: true,
        allocations: [{ component_id: 'food-1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }],
      },
      {
        member_id: 'morgan',
        display_name: 'Morgan Lee',
        status: 'planned',
        can_record: true,
        allocations: [{ component_id: 'food-1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }],
      },
    ],
    guest_groups: [{
      id: 'guests-1',
      count: 1,
      status: 'planned',
      allocations: [{ component_id: 'food-1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }],
    }],
    opted_out: [],
    can_opt_out: true,
    can_join: false,
    capabilities: { can_edit: true, can_delete: true, can_record_guests: true },
    revision: 2,
    ...overrides,
  };
}

function renderSheet(value: PlannerMeal) {
  render(
    <MealSheet
      meal={value}
      onClose={vi.fn()}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
      onLeave={vi.fn()}
      onJoin={vi.fn()}
    />,
  );
  return within(screen.getByRole('dialog'));
}

describe('MealSheet', () => {
  it('shows the meal schedule, diners, food, amounts and shortage', () => {
    const dialog = renderSheet(meal());

    expect(dialog.getByText('Chicken and rice, Yoghurt')).toBeInTheDocument();
    expect(dialog.getByText('Dinner · Wednesday 16 September · 18:00')).toBeInTheDocument();
    expect(dialog.getByText('Alex Brown')).toBeInTheDocument();
    expect(dialog.getByText('Morgan Lee')).toBeInTheDocument();
    expect(dialog.getByText('1 guest')).toBeInTheDocument();
    expect(dialog.getAllByText('1 serving')).toHaveLength(3);
    expect(dialog.getByText('Chicken and rice')).toBeInTheDocument();
    expect(dialog.getByText('Recipe')).toBeInTheDocument();
    expect(dialog.getByText('3 servings')).toBeInTheDocument();
    expect(dialog.getByText('Yoghurt')).toBeInTheDocument();
    expect(dialog.getByText('Product')).toBeInTheDocument();
    expect(dialog.getByText('150 g')).toBeInTheDocument();
    expect(dialog.getByText('Not enough servings for Chicken and rice')).toBeInTheDocument();
  });

  it('shows only actions allowed by the meal', () => {
    const dialog = renderSheet(meal());

    expect(dialog.getByRole('button', { name: 'Edit meal' })).toBeInTheDocument();
    expect(dialog.getByRole('button', { name: 'Delete meal' })).toBeInTheDocument();
    expect(dialog.getByRole('button', { name: 'Leave this meal' })).toBeInTheDocument();
    expect(dialog.queryByRole('button', { name: 'Join this meal' })).not.toBeInTheDocument();
  });

  it('shows join without management actions when only joining is allowed', () => {
    const dialog = renderSheet(meal({
      can_opt_out: false,
      can_join: true,
      capabilities: { can_edit: false, can_delete: false, can_record_guests: false },
    }));

    expect(dialog.getByRole('button', { name: 'Join this meal' })).toBeInTheDocument();
    expect(dialog.queryByRole('button', { name: 'Edit meal' })).not.toBeInTheDocument();
    expect(dialog.queryByRole('button', { name: 'Delete meal' })).not.toBeInTheDocument();
    expect(dialog.queryByRole('button', { name: 'Leave this meal' })).not.toBeInTheDocument();
  });
});
