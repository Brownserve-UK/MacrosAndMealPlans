import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { shoppingList } from './fixtures';
import { TripPage } from './TripPage';

beforeAll(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.setSystemTime(new Date('2026-09-01T09:00:00Z'));
});

afterAll(() => {
  vi.useRealTimers();
});

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
  useNavigate: () => vi.fn(),
  useParams: () => ({ date: '2026-09-05' }),
}));

const withManual = {
  ...shoppingList,
  manual: [
    {
      id: 'm1',
      name: 'Onion Salt',
      section: 'ambient' as const,
      opportunity_date: '2026-09-05',
      revision: 1,
    },
  ],
};

vi.mock('../../api/queries', () => ({
  useShoppingList: () => ({ isLoading: false, isError: false, data: withManual }),
  useStartShop: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useAbandonShop: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useFinishShop: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useRecordPurchase: () => ({ isPending: false, mutate: vi.fn(), mutateAsync: vi.fn() }),
  useUpdatePurchase: () => ({ isPending: false, mutate: vi.fn(), mutateAsync: vi.fn() }),
  useAddShoppingListItem: () => ({ isPending: false, mutate: vi.fn() }),
  useRemoveShoppingListItem: () => ({ isPending: false, mutate: vi.fn() }),
  useUpdateShoppingListItem: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useDismissShoppingSuggestion: () => ({ isPending: false, mutate: vi.fn() }),
  useProducts: () => ({ data: { items: [], total: 0 } }),
  useUnits: () => ({ data: ['g', 'ml'] }),
}));

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <TripPage />
    </QueryClientProvider>,
  );
}

describe('TripPage', () => {
  it('groups what to buy into aisles', () => {
    renderPage();

    const headings = screen.getAllByText(/Fresh produce|Meat & fish|Dairy|Ambient/);
    expect(headings.map((node) => node.textContent)).toEqual(['Dairy', 'Ambient']);
  });

  it('puts what you added by hand in its own aisle', () => {
    renderPage();

    const ambient = screen.getByText('Ambient').closest('div');
    expect(ambient).not.toBeNull();
    expect(ambient!.textContent).toContain('Onion Salt');
    expect(screen.queryByText('You added')).not.toBeInTheDocument();
  });

  it('lets you tick off what you added by hand', () => {
    renderPage();

    expect(screen.getByLabelText('Bought Onion Salt')).toBeInTheDocument();
  });

  it('shows the shelf life on the row itself', () => {
    renderPage();

    expect(screen.getByText(/Use by at least/)).toBeInTheDocument();
  });

  it('says which meal an item is for after its shelf life', () => {
    renderPage();

    expect(screen.getByText(/Use by at least .* · Porridge and 1 more/)).toBeInTheDocument();
  });

  it('shows an amount only where there is one worth showing', () => {
    renderPage();

    expect(screen.getByText('600 ml')).toBeInTheDocument();
    expect(screen.getByText('500 g')).toBeInTheDocument();
  });

  it('offers a way to start before anything is ticked', () => {
    renderPage();

    expect(screen.getByRole('button', { name: 'Start shopping' })).toBeInTheDocument();
  });

  it('lets you add anything without leaving the list', () => {
    renderPage();

    expect(screen.getByPlaceholderText('Add anything')).toBeInTheDocument();
  });

  it('says none of the internal vocabulary out loud', () => {
    renderPage();

    for (const leak of [/unassigned/i, /assumption_only/i, /incompatible_units/i]) {
      expect(screen.queryByText(leak)).not.toBeInTheDocument();
    }
  });
});
