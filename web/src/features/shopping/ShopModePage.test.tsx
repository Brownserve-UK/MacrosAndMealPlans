import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { shoppingList } from './fixtures';
import { ShopModePage } from './ShopModePage';

const navigate = vi.fn();
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => navigate,
}));

const record = vi.fn();
const recordAsync = vi.fn();
const update = vi.fn();
const updateAsync = vi.fn();
const finishAsync = vi.fn();

vi.mock('../../api/queries', () => ({
  useShoppingList: () => ({ isLoading: false, isError: false, data: shoppingList }),
  useProducts: () => ({
    data: { items: [{ id: 'butter-1', name: 'Salted butter' }], total: 1 },
  }),
  useRecordPurchase: () => ({ isPending: false, mutate: record, mutateAsync: recordAsync }),
  useUpdatePurchase: () => ({ isPending: false, mutate: update, mutateAsync: updateAsync }),
  useFinishShop: () => ({ isPending: false, mutateAsync: finishAsync }),
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
  navigate.mockReset();
  record.mockReset();
  recordAsync.mockReset();
  update.mockReset();
  updateAsync.mockReset();
  finishAsync.mockReset();
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

  it('unticking cancels every purchase on that row', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('checkbox', { name: 'Bought Butter' }));

    expect(update).toHaveBeenCalledTimes(2);
    expect(update).toHaveBeenCalledWith({ id: 'p1', revision: 1, cancelled: true });
    expect(update).toHaveBeenCalledWith({ id: 'p2', revision: 1, cancelled: true });
  });

  it('shows what is in the trolley, including a line with no details yet', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Butter'));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('In the trolley')).toBeInTheDocument();
    expect(within(dialog).getByText('Product not said yet')).toBeInTheDocument();
    expect(within(dialog).getByText('Amount not said yet')).toBeInTheDocument();
    expect(within(dialog).getByText('Salted butter')).toBeInTheDocument();
    expect(within(dialog).getByText('500 g')).toBeInTheDocument();
  });

  it('lets you change your mind about what you picked up', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Butter'));
    await user.click(screen.getByRole('button', { name: 'Change Salted butter' }));

    const form = screen.getByRole('dialog');
    const amount = within(form).getByLabelText('Amount');
    await user.clear(amount);
    await user.type(amount, '750');
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(updateAsync).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'p2', quantity: { amount: 750, unit: 'g' } }),
    );
  });

  it('lets you add a second product against one item', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Butter'));
    await user.click(screen.getByRole('button', { name: 'Add another' }));

    const form = screen.getByRole('dialog');
    expect(within(form).getByText('Add what you bought')).toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(recordAsync).toHaveBeenCalledWith(
      expect.objectContaining({ ingredient_id: 'butter', product_id: 'butter-1' }),
    );
  });

  it('removes a trolley line', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText('Butter'));
    await user.click(screen.getByRole('button', { name: 'Remove Salted butter' }));

    expect(updateAsync).toHaveBeenCalledWith({ id: 'p2', revision: 1, cancelled: true });
  });

  it('finishing the shop says what happens, then reconciles the focused shop', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('button', { name: 'Finish shop' }));

    const confirm = screen.getByRole('dialog');
    expect(confirm).toHaveTextContent('1 item goes into your stock.');
    expect(confirm).toHaveTextContent('1 still needs details and will wait.');

    await user.click(within(confirm).getByRole('button', { name: 'Finish' }));

    expect(finishAsync).toHaveBeenCalledWith('2026-09-05');
    expect(navigate).toHaveBeenCalledWith({ to: '/shopping' });
  });
});
