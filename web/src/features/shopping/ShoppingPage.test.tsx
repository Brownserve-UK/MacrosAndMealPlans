import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { shoppingList } from './fixtures';
import { ShoppingPage } from './ShoppingPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('../../api/queries', () => ({
  useShoppingList: () => ({ isLoading: false, isError: false, data: shoppingList }),
  useSkipShoppingOpportunity: () => ({ isPending: false, mutate: vi.fn() }),
  useProducts: () => ({ data: { items: [], total: 0 } }),
  useRecordPurchase: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useUpdatePurchase: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useUnits: () => ({ data: ['g', 'ml'] }),
}));

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ShoppingPage />
    </QueryClientProvider>,
  );
}

describe('ShoppingPage', () => {
  it('groups items by supermarket section in aisle order', () => {
    renderPage();

    const headings = screen.getAllByText(/Fresh produce|Meat & fish|Dairy|Ambient/);
    expect(headings.map((node) => node.textContent)).toEqual(['Dairy', 'Ambient']);
  });

  it('keeps an urgent item inside its own section rather than a banner', () => {
    renderPage();

    const ambient = screen.getByText('Ambient').closest('div')!.parentElement!;
    expect(within(ambient).getByText('Plain Flour')).toBeInTheDocument();
    expect(screen.queryByText(/needed before your next shop/i)).not.toBeInTheDocument();
  });

  it('keeps the card face clear of explanations', () => {
    renderPage();

    expect(screen.queryByText(/Use by at least/)).not.toBeInTheDocument();
    expect(screen.queryByText('Maybe')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Why' })).not.toBeInTheDocument();
  });

  it('opens the pop-up with a labelled headline and two distinguishable dates', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Whole Milk'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('Still to buy')).toBeInTheDocument();
    expect(within(dialog).getByText('600 ml')).toBeInTheDocument();
    expect(within(dialog).getByText('Needed by').parentElement).toHaveTextContent(
      /Needed byMon 7 Sept?/,
    );
    expect(within(dialog).getByText('Must keep until').parentElement).toHaveTextContent(
      /Must keep untilWed 9 Sept?/,
    );
  });

  it('lists the meals behind it', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Whole Milk'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('What needs it')).toBeInTheDocument();
    expect(within(dialog).getAllByText('Breakfast')).toHaveLength(2);
    expect(within(dialog).getAllByText('300 ml')).toHaveLength(2);
  });

  it('flags why something is needed sooner', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Plain Flour'));

    const alert = within(screen.getByRole('dialog')).getByRole('alert');
    expect(alert).toHaveTextContent('Needed before your next shop');
  });

  it('offers no way to buy from planning mode', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Whole Milk'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).queryByRole('button', { name: 'Add another' })).toBeNull();
  });
});
