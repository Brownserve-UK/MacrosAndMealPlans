import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { StockPage } from './StockPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

const exact = (n: number, unit: string) => ({ mode: 'exact' as const, quantity: { amount: n, unit } });
const quantified = (onHand: number, planned: number, unit: string, confidence = 'exact') => ({
  state: 'quantified' as const,
  confidence,
  on_hand: { amount: onHand, unit },
  planned_demand: { amount: planned, unit },
  unallocated: { amount: onHand - planned, unit },
});

vi.mock('../../api/queries', () => ({
  useStock: () => ({
    isLoading: false,
    isError: false,
    data: {
      items: [
        { id: 'br1', product_id: 'br', level: exact(120, 'g'), storage_location: 'chilled', revision: 1 },
        { id: 'mk1', product_id: 'mk', level: { mode: 'estimated', low: 300, high: 1500, unit: 'ml' }, storage_location: 'chilled', revision: 1 },
        { id: 'oa1', product_id: 'oa', level: exact(500, 'g'), storage_location: 'ambient', revision: 1 },
        { id: 'ck1', product_id: 'ck', level: exact(400, 'g'), storage_location: 'chilled', usability_deadline: { date: '2026-08-27' }, revision: 1 },
        { id: 'ck2', product_id: 'ck', level: exact(650, 'g'), storage_location: 'frozen', revision: 1 },
        { id: 'ri1', product_id: 'ri', level: { mode: 'not_tracked' }, storage_location: 'ambient', revision: 1 },
      ],
      total: 6,
    },
  }),
  useStockAvailability: () => ({
    data: [
      { product_id: 'br', demand_incomplete: false, availability: quantified(120, 300, 'g') },
      { product_id: 'mk', demand_incomplete: false, availability: quantified(300, 500, 'ml', 'estimated') },
      { product_id: 'oa', demand_incomplete: false, availability: quantified(500, 160, 'g') },
      { product_id: 'ck', demand_incomplete: false, availability: quantified(1050, 300, 'g') },
      { product_id: 'ri', demand_incomplete: false, availability: { state: 'assumed_available' } },
    ],
  }),
  useProducts: () => ({
    data: {
      items: [
        { id: 'br', name: 'Sample Broccoli' },
        { id: 'mk', name: 'Sample Whole Milk' },
        { id: 'oa', name: 'Sample Jumbo Oats' },
        { id: 'ck', name: 'Sample Chicken Breast' },
        { id: 'ri', name: 'Sample Basmati Rice' },
      ],
    },
  }),
  useCreateStockItem: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <StockPage />
    </QueryClientProvider>,
  );
}

const order = () =>
  screen.getAllByTestId(/^stock-card-/).map((node) => node.getAttribute('data-testid'));

describe('StockPage', () => {
  it('puts shortfalls first and shows needed / available in the tier colour', () => {
    renderPage();
    expect(order()[0]).toBe('stock-card-br');
    const broccoli = screen.getByTestId('stock-card-br');
    expect(within(broccoli).getByText('300 g / 120 g')).toBeInTheDocument();
  });

  it('marks an estimated available figure with a tilde', () => {
    renderPage();
    const milk = screen.getByTestId('stock-card-mk');
    expect(within(milk).getByText('500 ml / ~300 ml')).toBeInTheDocument();
  });

  it('shows a healthy item with its figure and no alarm', () => {
    renderPage();
    const oats = screen.getByTestId('stock-card-oa');
    expect(within(oats).getByText('160 g / 500 g')).toBeInTheDocument();
  });

  it('collapses multiple lots of one product into one card', () => {
    renderPage();
    expect(screen.getAllByText('Sample Chicken Breast')).toHaveLength(1);
    const chicken = screen.getByTestId('stock-card-ck');
    expect(within(chicken).getByText('2 lots')).toBeInTheDocument();
    expect(within(chicken).getByText('300 g / 1,050 g')).toBeInTheDocument();
  });

  it('shows a not-tracked staple as assumed available, with no figure, and sorts it last', () => {
    renderPage();
    const rice = screen.getByTestId('stock-card-ri');
    expect(within(rice).getByText('Assumed available')).toBeInTheDocument();
    expect(within(rice).queryByText(/\//)).toBeNull();
    expect(order().at(-1)).toBe('stock-card-ri');
  });

  it('filters the list by product name', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.type(screen.getByPlaceholderText('Search stock'), 'milk');
    await waitFor(() => expect(order()).toEqual(['stock-card-mk']));
  });

  it('reorders when the sort chip changes', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(screen.getByRole('button', { name: 'Name' }));
    expect(order()).toEqual([
      'stock-card-ri',
      'stock-card-br',
      'stock-card-ck',
      'stock-card-oa',
      'stock-card-mk',
    ]);
  });
});
