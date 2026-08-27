import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Product, Recipe, UnitInfo } from '../../api/client';
import { RecipePage } from './RecipePage';

const mocks = vi.hoisted(() => ({
  preview: vi.fn(),
  update: vi.fn(),
  archive: vi.fn(),
  units: undefined as UnitInfo[] | undefined,
}));

const units: UnitInfo[] = [
  { code: 'mg', label: 'milligram', dimension: 'mass', convertible: true },
  { code: 'g', label: 'gram', dimension: 'mass', convertible: true },
  { code: 'kg', label: 'kilogram', dimension: 'mass', convertible: true },
];

const product: Product = {
  id: 'p1',
  name: 'Whole Milk',
  package_quantity: { amount: 600, unit: 'g' },
  servings_per_pack: 4,
  nutrition: { basis: { amount: 100, unit: 'g' }, energy_kcal: 64 },
  provenance: { origin: 'local', locally_modified: false },
  revision: 1,
  created_at: '2026-08-24T10:00:00Z',
  updated_at: '2026-08-24T10:00:00Z',
} as unknown as Product;

const recipe: Recipe = {
  id: 'r1',
  name: 'Warm Milk',
  servings: 2,
  components: [{ id: 'c1', product_id: 'p1', amount: { kind: 'measure', value: 100, unit: 'g' }, position: 0 }],
  owner_id: 'user-1',
  visibility: 'private',
  created_by: 'user-1',
  updated_by: 'user-1',
  revision: 4,
  created_at: '2026-08-24T10:00:00Z',
  updated_at: '2026-08-24T10:00:00Z',
};

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
  useNavigate: () => vi.fn(),
}));

vi.mock('../../api/queries', () => ({
  useRecipe: () => ({ data: recipe, isLoading: false, isError: false, refetch: vi.fn() }),
  useRecipeNutrition: () => ({
    data: { nutrition: { energy_kcal: 32, protein_g: 3.2 }, quality: 'known' },
  }),
  useRecipeNutritionPreview: () => ({ mutate: mocks.preview, data: undefined, isPending: false }),
  useUpdateRecipe: () => ({ mutateAsync: mocks.update, isPending: false }),
  useSetRecipeArchived: () => ({ mutateAsync: mocks.archive, isPending: false }),
  useProduct: () => ({ data: product, isLoading: false }),
  useProducts: () => ({ data: { items: [product] } }),
  useUnits: () => ({ data: mocks.units }),
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <RecipePage id="r1" />
    </QueryClientProvider>,
  );
}

describe('RecipePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.units = units;
  });

  it('shows the recipe and its per-serving nutrition', () => {
    renderPage();
    expect(screen.getByRole('heading', { name: 'Warm Milk' })).toBeInTheDocument();
    expect(screen.getByText('32 kcal')).toBeInTheDocument();
  });

  it('asks the server to derive nutrition for the current draft', async () => {
    renderPage();
    await waitFor(() => expect(mocks.preview).toHaveBeenCalled());
    expect(mocks.preview).toHaveBeenLastCalledWith(
      expect.objectContaining({
        servings: 2,
        components: [
          expect.objectContaining({
            id: 'c1',
            product_id: 'p1',
            amount: { kind: 'measure', value: 100, unit: 'g' },
          }),
        ],
      }),
    );
  });

  it('keeps a stored measured amount when products load before units', async () => {
    mocks.units = undefined;
    const page = renderPage();

    expect(screen.getByLabelText('Unit')).toHaveAttribute('aria-disabled', 'true');

    mocks.units = units;
    page.rerender(
      <QueryClientProvider client={new QueryClient()}>
        <RecipePage id="r1" />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(screen.getByLabelText('Unit')).toHaveTextContent('g'));
    expect(screen.queryByText('1 serving is 150 g')).not.toBeInTheDocument();
  });
});
