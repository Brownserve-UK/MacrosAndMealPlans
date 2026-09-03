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
        { id: 'mk1', product_id: 'mk', level: { mode: 'estimated', quantity: { amount: 300, unit: 'ml' } }, storage_location: 'chilled', revision: 1 },
        { id: 'mv1', product_id: 'mv', level: exact(450, 'ml'), storage_location: 'chilled', revision: 1 },
        { id: 'oa1', product_id: 'oa', level: exact(500, 'g'), storage_location: 'ambient', revision: 1 },
        { id: 'ck1', product_id: 'ck', level: exact(400, 'g'), storage_location: 'chilled', usability_deadline: { date: '2026-08-27' }, revision: 1 },
        { id: 'ck2', product_id: 'ck', level: exact(650, 'g'), storage_location: 'frozen', revision: 1 },
        { id: 'ri1', product_id: 'ri', level: { mode: 'not_tracked' }, storage_location: 'ambient', revision: 1 },
      ],
      total: 7,
    },
  }),
  useStockAvailability: () => ({
    data: {
      products: [
        { product_id: 'br', demand_gaps: [], availability: quantified(120, 300, 'g') },
        { product_id: 'mk', demand_gaps: [], availability: quantified(300, 500, 'ml', 'estimated') },
        { product_id: 'mv', demand_gaps: [], availability: quantified(450, 0, 'ml') },
        { product_id: 'oa', demand_gaps: [], availability: quantified(500, 160, 'g') },
        { product_id: 'ck', demand_gaps: [], availability: quantified(1050, 300, 'g') },
        { product_id: 'ri', demand_gaps: [], availability: { state: 'assumed_available' } },
      ],
      ingredients: [
        { ingredient_id: 'milk', name: 'Whole Milk', demand_gaps: [], availability: quantified(750, 500, 'ml') },
        { ingredient_id: 'oats', name: 'Jumbo Oats', demand_gaps: [], availability: quantified(500, 160, 'g') },
        { ingredient_id: 'chicken', name: 'Chicken Breast', demand_gaps: [], availability: quantified(1050, 300, 'g') },
        { ingredient_id: 'rice', name: 'Basmati Rice', demand_gaps: [], availability: { state: 'assumed_available' } },
      ],
      demand_gaps: [],
    },
  }),
  useProducts: () => ({
    data: {
      items: [
        { id: 'br', name: 'Sample Broccoli' },
        { id: 'mk', name: 'Sample Whole Milk', mapped_ingredient_id: 'milk' },
        { id: 'mv', name: 'Sample Value Milk', mapped_ingredient_id: 'milk' },
        { id: 'oa', name: 'Sample Jumbo Oats', mapped_ingredient_id: 'oats' },
        { id: 'ck', name: 'Sample Chicken Breast', mapped_ingredient_id: 'chicken' },
        { id: 'ri', name: 'Sample Basmati Rice', mapped_ingredient_id: 'rice' },
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

const productOrder = () =>
  screen.getAllByTestId(/^stock-card-/).map((node) => node.getAttribute('data-testid'));
const ingredientOrder = () =>
  screen.getAllByTestId(/^stock-ingredient-/).map((node) => node.getAttribute('data-testid'));

async function showProducts() {
  await userEvent.setup().click(screen.getByRole('tab', { name: 'Products' }));
}

describe('StockPage ingredients view', () => {
  it('opens on ingredients rather than products', () => {
    renderPage();
    expect(screen.getByRole('tab', { name: 'Ingredients' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(screen.queryAllByTestId(/^stock-card-/)).toHaveLength(0);
    expect(ingredientOrder().length).toBeGreaterThan(0);
  });

  it('pools the products of one ingredient into a single row', () => {
    renderPage();
    const milk = screen.getByTestId('stock-ingredient-milk');
    expect(within(milk).getByText('Whole Milk')).toBeInTheDocument();
    expect(within(milk).getByText('2 products')).toBeInTheDocument();
    expect(within(milk).getByText('500 ml / 750 ml')).toBeInTheDocument();
    expect(screen.queryByText('Sample Whole Milk')).toBeNull();
  });

  it('says "1 product" when only one product stands in for the ingredient', () => {
    renderPage();
    const oats = screen.getByTestId('stock-ingredient-oats');
    expect(within(oats).getByText('1 product')).toBeInTheDocument();
  });

  it('leaves a product with no ingredient out of the ingredients view', () => {
    renderPage();
    expect(screen.queryByText('Sample Broccoli')).toBeNull();
    expect(ingredientOrder()).not.toContain('stock-ingredient-br');
  });

  it('puts shortfalls first here too', () => {
    renderPage();
    expect(ingredientOrder()[0]).toBe('stock-ingredient-milk');
  });

  it('filters by ingredient name', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.type(screen.getByPlaceholderText('Search stock'), 'milk');
    await waitFor(() => expect(ingredientOrder()).toEqual(['stock-ingredient-milk']));
  });
});

describe('StockPage products view', () => {
  it('puts shortfalls first and shows needed / available in the tier colour', async () => {
    renderPage();
    await showProducts();
    expect(productOrder()[0]).toBe('stock-card-br');
    const broccoli = screen.getByTestId('stock-card-br');
    expect(within(broccoli).getByText('300 g / 120 g')).toBeInTheDocument();
  });

  it('marks an estimated available figure with a tilde', async () => {
    renderPage();
    await showProducts();
    const milk = screen.getByTestId('stock-card-mk');
    expect(within(milk).getByText('500 ml / ~300 ml')).toBeInTheDocument();
  });

  it('shows a healthy item with its figure and no alarm', async () => {
    renderPage();
    await showProducts();
    const oats = screen.getByTestId('stock-card-oa');
    expect(within(oats).getByText('160 g / 500 g')).toBeInTheDocument();
  });

  it('collapses multiple lots of one product into one card', async () => {
    renderPage();
    await showProducts();
    expect(screen.getAllByText('Sample Chicken Breast')).toHaveLength(1);
    const chicken = screen.getByTestId('stock-card-ck');
    expect(within(chicken).getByText('chilled, frozen · nearest date 27/08/2026')).toBeInTheDocument();
    expect(within(chicken).getByText('300 g / 1,050 g')).toBeInTheDocument();
  });

  it('shows a not-tracked staple as assumed available, with no figure, and sorts it last', async () => {
    renderPage();
    await showProducts();
    const rice = screen.getByTestId('stock-card-ri');
    expect(within(rice).getByText('Assumed available')).toBeInTheDocument();
    expect(within(rice).queryByText(/\//)).toBeNull();
    expect(productOrder().at(-1)).toBe('stock-card-ri');
  });

  it('filters the list by product name', async () => {
    const user = userEvent.setup();
    renderPage();
    await showProducts();
    await user.type(screen.getByPlaceholderText('Search stock'), 'broccoli');
    await waitFor(() => expect(productOrder()).toEqual(['stock-card-br']));
  });

  it('reorders when the sort selection changes', async () => {
    const user = userEvent.setup();
    renderPage();
    await showProducts();
    await user.click(screen.getByRole('combobox', { name: 'Sort' }));
    await user.click(within(screen.getByRole('listbox')).getByText('Name'));
    expect(productOrder()).toEqual([
      'stock-card-ri',
      'stock-card-br',
      'stock-card-ck',
      'stock-card-oa',
      'stock-card-mv',
      'stock-card-mk',
    ]);
  });

  it('no longer claims recipe meals are uncounted', () => {
    renderPage();
    expect(screen.queryByText(/aren't counted against stock yet/)).toBeNull();
  });
});
