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

  it('explains a requirement in the pop-up, with the meals behind it', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Whole Milk'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText(/Needed by Mon 7 Sept?/)).toBeInTheDocument();
    expect(within(dialog).getByText(/Use by at least Wed 9 Sept?/)).toBeInTheDocument();
    const lines = within(dialog)
      .getAllByText((_, node) => (node?.textContent ?? '').includes('breakfast'))
      .map((node) => node.textContent);
    expect(lines.some((line) => line?.includes('Mon 7 Sep') && line.includes('300 ml'))).toBe(true);
  });

  it('says in words why something is needed sooner', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Plain Flour'));

    expect(
      within(screen.getByRole('dialog')).getByText('Needed before your next shop.'),
    ).toBeInTheDocument();
  });

  it('offers no way to buy from planning mode', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Whole Milk'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).queryByRole('button', { name: /Mark bought|Add to stock/ })).toBeNull();
  });
});
