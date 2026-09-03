import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { IngredientsPage } from './IngredientsPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
  useNavigate: () => vi.fn(),
}));

const calls: Record<string, unknown>[] = [];

vi.mock('../../api/queries', () => ({
  useIngredients: (params: Record<string, unknown>) => {
    calls.push(params);
    return {
      isLoading: false,
      isError: false,
      data: {
        items: [
          { id: 'a', name: 'Basmati Rice', mapped_product_count: 2 },
          { id: 'b', name: 'Ground Cinnamon', mapped_product_count: 0 },
        ],
        total: 2,
      },
    };
  },
  useCreateIngredient: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

function renderPage() {
  calls.length = 0;
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <IngredientsPage />
    </QueryClientProvider>,
  );
}

const latest = () => calls[calls.length - 1];

describe('IngredientsPage', () => {
  it('sorts A-Z by default', () => {
    renderPage();
    expect(latest()).toMatchObject({ sort_by: 'name', sort: 'asc' });
    expect(screen.getByRole('combobox', { name: 'Sort' })).toHaveTextContent('A-Z');
  });

  it('offers date added and product count, newest and most first', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('combobox', { name: 'Sort' }));
    await user.click(within(screen.getByRole('listbox')).getByText('Date added'));
    expect(latest()).toMatchObject({ sort_by: 'created', sort: 'desc' });

    await user.click(screen.getByRole('combobox', { name: 'Sort' }));
    await user.click(within(screen.getByRole('listbox')).getByText('Product count'));
    expect(latest()).toMatchObject({ sort_by: 'product_count', sort: 'desc' });
  });

  it('goes back to the first page when the sort changes', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('combobox', { name: 'Sort' }));
    await user.click(within(screen.getByRole('listbox')).getByText('Date added'));
    expect(latest()).toMatchObject({ page: 1 });
  });
});
