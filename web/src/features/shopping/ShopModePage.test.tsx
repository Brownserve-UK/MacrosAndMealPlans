import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { shoppingList } from './fixtures';
import { ShopModePage } from './ShopModePage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

const record = vi.fn();
const recordAsync = vi.fn();
const update = vi.fn();
const updateAsync = vi.fn();

vi.mock('../../api/queries', () => ({
  useShoppingList: () => ({ isLoading: false, isError: false, data: shoppingList }),
  useProducts: () => ({ data: { items: [{ id: 'butter-1', name: 'Salted butter' }], total: 1 } }),
  useRecordPurchase: () => ({ isPending: false, mutate: record, mutateAsync: recordAsync }),
  useUpdatePurchase: () => ({ isPending: false, mutate: update, mutateAsync: updateAsync }),
  useUnits: () => ({ data: ['g', 'ml'] }),
}));

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ShopModePage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  record.mockReset();
  recordAsync.mockReset();
  update.mockReset();
  updateAsync.mockReset();
});

describe('ShopModePage', () => {
  it('counts what is already in the trolley', () => {
    renderPage();

    expect(screen.getByText('1 of 3 in the trolley')).toBeInTheDocument();
  });

  it('ticking an item records it as bought without inventing any details', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('checkbox', { name: 'Bought Whole Milk' }));

    expect(record).toHaveBeenCalledWith({
      ingredient_id: 'milk',
      product_id: undefined,
      opportunity_date: '2026-09-05',
    });
  });

  it('unticking a bought item cancels the purchase', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('checkbox', { name: 'Bought Butter' }));

    expect(update).toHaveBeenCalledWith({ id: 'p1', revision: 1, cancelled: true });
  });

  it('opening a bought item offers the details of what was bought', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Butter'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByLabelText('Amount')).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Add to stock' })).toBeInTheDocument();
  });

  it('sends the amount actually bought, not the amount needed', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Butter'));
    const dialog = screen.getByRole('dialog');
    const amount = within(dialog).getByLabelText('Amount');
    await user.clear(amount);
    await user.type(amount, '500');
    await user.click(within(dialog).getByRole('button', { name: 'Add to stock' }));

    expect(updateAsync).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'p1', quantity: { amount: 500, unit: 'g' } }),
    );
  });

  it('leaves an unbought item read-only until it is ticked', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Whole Milk'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).queryByLabelText('Amount')).toBeNull();
  });
});
