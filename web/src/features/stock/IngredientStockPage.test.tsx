import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IngredientStockPage } from './IngredientStockPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

const exact = (n: number, unit: string) => ({ mode: 'exact' as const, quantity: { amount: n, unit } });
const quantified = (onHand: number, planned: number, unit: string) => ({
  state: 'quantified' as const,
  confidence: 'exact',
  on_hand: { amount: onHand, unit },
  planned_demand: { amount: planned, unit },
  unallocated: { amount: onHand - planned, unit },
});

vi.mock('../../api/queries', () => ({
  useIngredient: () => ({
    isLoading: false,
    isError: false,
    data: { id: 'milk', name: 'Whole Milk' },
  }),
  useIngredientProducts: () => ({
    isLoading: false,
    isError: false,
    data: {
      items: [
        { id: 'mk', name: 'Sample Whole Milk' },
        { id: 'mv', name: 'Sample Value Milk' },
        { id: 'mn', name: 'Sample Milk We Have None Of' },
      ],
    },
  }),
  useStock: () => ({
    isLoading: false,
    isError: false,
    data: {
      items: [
        { id: 'mk1', product_id: 'mk', level: exact(300, 'ml'), storage_location: 'chilled', revision: 1 },
        { id: 'mv1', product_id: 'mv', level: exact(450, 'ml'), storage_location: 'chilled', usability_deadline: { date: '2026-09-08' }, revision: 1 },
      ],
      total: 2,
    },
  }),
  useStockAvailability: () => ({
    isLoading: false,
    isError: false,
    data: {
      products: [
        { product_id: 'mk', demand_gaps: [], availability: quantified(300, 400, 'ml') },
        { product_id: 'mv', demand_gaps: [], availability: quantified(450, 100, 'ml') },
      ],
      ingredients: [
        { ingredient_id: 'milk', name: 'Whole Milk', demand_gaps: [], availability: quantified(750, 500, 'ml') },
      ],
      demand_gaps: [],
    },
  }),
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <IngredientStockPage ingredientId="milk" />
    </QueryClientProvider>,
  );
}

describe('IngredientStockPage', () => {
  it('shows the pooled availability for the ingredient', () => {
    renderPage();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
    expect(screen.getByText('500 ml / 750 ml')).toBeInTheDocument();
    expect(screen.getByText('750 ml on hand · 500 ml planned')).toBeInTheDocument();
  });

  it('lists the products we actually hold, with their own figures', () => {
    renderPage();
    const value = screen.getByTestId('stock-card-mv');
    expect(within(value).getByText('Sample Value Milk')).toBeInTheDocument();
    expect(within(value).getByText('100 ml / 450 ml')).toBeInTheDocument();
    expect(within(value).getByText('chilled · nearest date 08/09/2026')).toBeInTheDocument();
  });

  it('leaves out a pool member we hold no stock of', () => {
    renderPage();
    expect(screen.queryByText('Sample Milk We Have None Of')).toBeNull();
  });
});
