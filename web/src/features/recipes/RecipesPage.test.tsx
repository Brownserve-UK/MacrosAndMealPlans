import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RecipesPage } from './RecipesPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('../../api/queries', () => ({
  useRecipes: () => ({
    data: {
      items: [{
        id: 'r1',
        name: 'Tomato Soup',
        description: 'A simple warming soup.',
        servings: 4,
        preparation_minutes: 10,
        cooking_minutes: 20,
        component_count: 3,
        meal_categories: ['lunch'],
        country_categories: ['IT'],
        tags: ['Quick'],
        revision: 1,
        updated_at: '2026-08-27T10:00:00Z',
      }],
      total: 1,
    },
    isLoading: false,
    isError: false,
  }),
}));

describe('RecipesPage', () => {
  it('renders a responsive recipe card with its warm fallback and metadata', () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <RecipesPage />
      </QueryClientProvider>,
    );

    expect(screen.getByRole('heading', { name: 'Tomato Soup' })).toBeInTheDocument();
    expect(screen.getByText('T')).toBeInTheDocument();
    expect(screen.getByText('Serves 4')).toBeInTheDocument();
    expect(screen.getByText('30 min total')).toBeInTheDocument();
    expect(screen.getByText('Lunch')).toBeInTheDocument();
    expect(screen.getByText(/Italy/)).toBeInTheDocument();
  });
});
