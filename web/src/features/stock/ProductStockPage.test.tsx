import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ProductStockPage } from './ProductStockPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('./NewStockDialog', () => ({ NewStockDialog: () => null }));

const quantified = (onHand: number, planned: number, unit: string) => ({
  state: 'quantified' as const,
  confidence: 'estimated',
  on_hand: { amount: onHand, unit },
  planned_demand: { amount: planned, unit },
  unallocated: { amount: onHand - planned, unit },
});

vi.mock('../../api/queries', () => ({
  useProduct: () => ({
    isLoading: false,
    isError: false,
    data: { id: 'mk', name: 'Sample Whole Milk', mapped_ingredient_id: 'milk' },
  }),
  useProducts: () => ({
    isLoading: false,
    isError: false,
    data: {
      items: [
        { id: 'mk', name: 'Sample Whole Milk' },
        { id: 'mv', name: 'Sample Value Whole Milk' },
      ],
    },
  }),
  useStock: () => ({
    isLoading: false,
    isError: false,
    data: {
      items: [
        {
          id: 'mk1',
          product_id: 'mk',
          level: { mode: 'estimated', quantity: { amount: 300, unit: 'ml' } },
          storage_location: 'chilled',
          revision: 1,
        },
      ],
      total: 1,
    },
  }),
  useStockAvailability: () => ({
    isLoading: false,
    isError: false,
    data: {
      products: [{ product_id: 'mk', demand_gaps: [], availability: quantified(450, 650, 'ml') }],
      ingredients: [
        {
          ingredient_id: 'milk',
          name: 'Whole Milk',
          demand_gaps: [],
          availability: quantified(1050, 900, 'ml'),
        },
      ],
      demand_gaps: [],
      claims: [
        {
          subject: { kind: 'product', product_id: 'mk' },
          quantity: { amount: 250, unit: 'ml' },
          entry_id: 'e1',
          planned_on: '2026-09-03',
          slot: 'breakfast',
          scope: 'member',
        },
        {
          subject: { kind: 'product', product_id: 'mk' },
          quantity: { amount: 250, unit: 'ml' },
          entry_id: 'e2',
          planned_on: '2026-09-04',
          slot: 'breakfast',
          scope: 'member',
        },
        {
          subject: { kind: 'ingredient', ingredient_id: 'milk' },
          quantity: { amount: 400, unit: 'ml' },
          entry_id: 'e3',
          planned_on: '2026-09-05',
          slot: 'breakfast',
          scope: 'member',
          recipe_name: 'Morning Porridge',
        },
      ],
    },
  }),
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <ProductStockPage productId="mk" />
    </QueryClientProvider>,
  );
}

describe('ProductStockPage', () => {
  it('separates the meals pinned to this product from the ones that asked for the ingredient', () => {
    renderPage();
    expect(screen.getByText('Planned for this product')).toBeInTheDocument();
    expect(screen.getByText('Breakfast · Thursday 3 September')).toBeInTheDocument();
    expect(screen.getByText('Could come from here or elsewhere')).toBeInTheDocument();
    expect(screen.getByText('Morning Porridge')).toBeInTheDocument();
  });

  it('says how much of the shared want is expected from here, and what else could supply it', () => {
    renderPage();
    expect(
      screen.getByText('We expect 150 ml of that to come from here.', { exact: false }),
    ).toBeInTheDocument();
    expect(screen.getByText('Sample Value Whole Milk', { exact: false })).toBeInTheDocument();
  });

  it('marks an estimated lot as approximate rather than precise', () => {
    renderPage();
    expect(screen.getByText('About 300 ml')).toBeInTheDocument();
  });
});
