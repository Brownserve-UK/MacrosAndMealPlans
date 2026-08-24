import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MealPlanEntryDialog } from './MealPlanEntryDialog';

vi.mock('../../api/queries', () => ({
  useCreateMealPlanEntry: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useDeleteMealPlanEntry: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useMarkMealPlanEaten: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useMarkMealPlanNotEaten: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useProduct: () => ({ data: null }),
  useProducts: () => ({ data: { items: [] }, isLoading: false }),
  useUpdateMealPlanEntry: () => ({ isPending: false, mutateAsync: vi.fn() }),
}));

describe('MealPlanEntryDialog', () => {
  it('offers to add a new planned meal', () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <MealPlanEntryDialog
          open
          onClose={vi.fn()}
          date="2026-08-24"
          slot="breakfast"
          entry={null}
        />
      </QueryClientProvider>,
    );

    expect(screen.getByRole('button', { name: 'Add to plan' })).toBeInTheDocument();
  });
});
