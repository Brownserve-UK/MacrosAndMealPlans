import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Product, Recipe, UnitInfo } from '../../api/client';
import { EditRecipePage } from './RecipeFormPage';

const mocks = vi.hoisted(() => ({
  update: vi.fn(),
  upload: vi.fn(),
  removePhoto: vi.fn(),
  navigate: vi.fn(),
  photoVersion: null as number | null,
}));

const product = {
  id: 'p1',
  name: 'Whole Milk',
  package_quantity: { amount: 1, unit: 'l' },
  servings_per_pack: 4,
  nutrition: { basis: { amount: 100, unit: 'ml' }, energy_kcal: 64 },
  provenance: { origin: 'local', locally_modified: false },
  revision: 1,
  created_at: '2026-08-24T10:00:00Z',
  updated_at: '2026-08-24T10:00:00Z',
} as Product;

const units: UnitInfo[] = [
  { code: 'ml', label: 'millilitre', dimension: 'volume', convertible: true },
  { code: 'l', label: 'litre', dimension: 'volume', convertible: true },
];

function recipe(): Recipe {
  return {
    id: 'r1',
    name: 'Warm Milk',
    servings: 2,
    components: [{ id: 'c1', product_id: 'p1', product_name: 'Whole Milk', amount: { kind: 'measure', value: 100, unit: 'ml' }, position: 0 }],
    instructions: [
      { id: 's1', text: 'First step', position: 0 },
      { id: 's2', text: 'Second step', position: 1 },
    ],
    meal_categories: ['snack'],
    country_categories: ['GB'],
    tags: ['Quick'],
    photo_version: mocks.photoVersion,
    owner_id: 'user-1',
    visibility: 'private',
    created_by: 'user-1',
    updated_by: 'user-1',
    revision: 4,
    created_at: '2026-08-24T10:00:00Z',
    updated_at: '2026-08-24T10:00:00Z',
  };
}

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
  useNavigate: () => mocks.navigate,
  useBlocker: () => ({ status: 'idle' }),
}));

vi.mock('../../api/queries', () => ({
  useRecipe: () => ({ data: recipe(), isLoading: false, isError: false, refetch: vi.fn() }),
  useCreateRecipe: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateRecipe: () => ({ mutateAsync: mocks.update, isPending: false }),
  useUploadRecipePhoto: () => ({ mutateAsync: mocks.upload, isPending: false }),
  useDeleteRecipePhoto: () => ({ mutateAsync: mocks.removePhoto, isPending: false }),
  useRecipePhoto: () => ({ data: undefined }),
  useProduct: () => ({ data: product, isLoading: false }),
  useProducts: () => ({ data: { items: [product] }, isLoading: false }),
  useUnits: () => ({ data: units }),
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <EditRecipePage id="r1" />
    </QueryClientProvider>,
  );
}

describe('RecipeFormPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.photoVersion = null;
    mocks.update.mockImplementation(async ({ body }) => ({ ...recipe(), ...body, revision: 5 }));
    Object.defineProperty(URL, 'createObjectURL', { value: vi.fn(() => 'blob:test'), configurable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: vi.fn(), configurable: true });
  });

  it('submits instructions in their edited order', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('button', { name: 'Move step 2 up' }));
    await user.click(screen.getByRole('button', { name: 'Save recipe' }));

    await waitFor(() => expect(mocks.update).toHaveBeenCalled());
    const request = mocks.update.mock.calls.at(0)![0];
    expect(request.body.instructions.map((step: { text: string }) => step.text)).toEqual(['Second step', 'First step']);
  });

  it('removes an existing photo after saving the recipe data', async () => {
    mocks.photoVersion = 2;
    mocks.update.mockImplementation(async ({ body }) => ({ ...recipe(), ...body, photo_version: 2, revision: 5 }));
    mocks.removePhoto.mockResolvedValue({ ...recipe(), photo_version: null, revision: 6 });
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('button', { name: 'Remove photo' }));
    await user.click(screen.getByRole('button', { name: 'Save recipe' }));

    await waitFor(() => expect(mocks.removePhoto).toHaveBeenCalledWith({ id: 'r1', revision: 5 }));
  });

  it('keeps a selected photo available when its upload fails', async () => {
    mocks.upload.mockRejectedValue(new Error('failed'));
    const user = userEvent.setup();
    renderPage();
    const input = document.querySelector<HTMLInputElement>('input[type="file"]');
    expect(input).not.toBeNull();

    await user.upload(input!, new File(['photo'], 'meal.png', { type: 'image/png' }));
    await user.click(screen.getByRole('button', { name: 'Save recipe' }));

    expect(await screen.findByText(/Recipe saved, but the photo was not changed/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Replace photo' })).toBeInTheDocument();
  });
});
