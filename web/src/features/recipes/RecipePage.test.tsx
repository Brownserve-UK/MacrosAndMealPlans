import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Recipe } from '../../api/client';
import { RecipePage } from './RecipePage';

const recipe: Recipe = {
  id: 'r1',
  name: 'Warm Milk',
  description: 'A quiet evening drink.',
  servings: 2,
  preparation_minutes: 5,
  cooking_minutes: 10,
  notes: 'Best served straight away.',
  components: [{ id: 'c1', product_id: 'p1', product_name: 'Whole Milk', amount: { kind: 'measure', value: 100, unit: 'g' }, position: 0 }],
  instructions: [{ id: 's1', text: 'Warm the milk gently.', position: 0 }],
  meal_categories: ['snack'],
  country_categories: ['GB'],
  tags: ['Cosy'],
  owner_id: 'user-1',
  visibility: 'private',
  created_by: 'user-1',
  updated_by: 'user-1',
  revision: 4,
  created_at: '2026-08-24T10:00:00Z',
  updated_at: '2026-08-24T10:00:00Z',
};

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('../../api/queries', () => ({
  useRecipe: () => ({ data: recipe, isLoading: false, isError: false, refetch: vi.fn() }),
  useRecipeNutrition: () => ({ data: { nutrition: { energy_kcal: 32, protein_g: 3.2 }, quality: 'known' } }),
  useSetRecipeArchived: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <RecipePage id="r1" />
    </QueryClientProvider>,
  );
}

describe('RecipePage', () => {
  it('renders an editorial recipe view with Products before instructions', () => {
    renderPage();
    expect(screen.getByRole('heading', { name: 'Warm Milk' })).toBeInTheDocument();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
    expect(screen.getByText('Warm the milk gently.')).toBeInTheDocument();
    expect(screen.getByText('Serves')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('Prep Time')).toBeInTheDocument();
    expect(screen.getByText('Cook Time')).toBeInTheDocument();
    expect(screen.getByText('Total Time')).toBeInTheDocument();
    expect(screen.getByTestId('RestaurantOutlinedIcon')).toBeInTheDocument();
    expect(screen.getByTestId('SoupKitchenOutlinedIcon')).toBeInTheDocument();
    expect(screen.getByTestId('AccessTimeOutlinedIcon')).toBeInTheDocument();
    expect(screen.getByText('5 min')).toBeInTheDocument();
    expect(screen.getByText('10 min')).toBeInTheDocument();
    expect(screen.getByText('15 min')).toBeInTheDocument();
    expect(screen.getAllByText(/^(Total|Prep|Cook) Time$/).map((element) => element.textContent)).toEqual([
      'Total Time',
      'Prep Time',
      'Cook Time',
    ]);
    expect(screen.getByText('32 kcal')).toBeInTheDocument();
  });

  it('shows notes and custom tags with the recipe metadata', () => {
    renderPage();
    expect(screen.getByText('Best served straight away.')).toBeInTheDocument();
    expect(screen.getByText('Cosy')).toBeInTheDocument();
  });
});
