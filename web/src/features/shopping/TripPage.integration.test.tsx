import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Purchase, ShoppingList, ShoppingTrip } from '../../api/client';
import { shoppingList } from './fixtures';
import { TripPage } from './TripPage';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
  useNavigate: () => vi.fn(),
  useParams: () => ({ date: '2026-09-05' }),
}));

const api = vi.hoisted(() => ({
  trip: undefined as ShoppingTrip | undefined,
  purchase: undefined as Purchase | undefined,
  requests: [] as string[],
}));

vi.mock('../../api/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api/client')>();
  const ok = (data: unknown, status = 200) => ({
    data,
    response: new Response(null, { status }),
  });
  return {
    ...actual,
    client: {
      GET: vi.fn(async (path: string) => {
        api.requests.push(`GET ${path}`);
        if (path === '/api/v1/shopping/requirements') {
          const requirement = { ...shoppingList.requirements[0]!, purchases: [] };
          const list: ShoppingList = {
            ...shoppingList,
            requirements: [
              { ...requirement, purchases: api.purchase ? [api.purchase] : [] },
            ],
            trip: api.trip,
          };
          return ok(list);
        }
        return ok({ items: [], page: 1, per_page: 25, total: 0, total_pages: 0 });
      }),
      POST: vi.fn(async (path: string) => {
        api.requests.push(`POST ${path}`);
        if (path.endsWith('/start')) {
          api.trip = {
            id: 'trip-1',
            opportunity_date: '2026-09-05',
            state: 'shopping',
            started_at: '2026-09-05T09:00:00Z',
            rows: [
              {
                id: 'row-1',
                ingredient_id: 'milk',
                name: 'Whole Milk',
                section: 'dairy',
              },
            ],
            revision: 1,
          };
          return ok(api.trip);
        }
        api.purchase = {
          id: 'purchase-1',
          ingredient_id: 'milk',
          opportunity_date: '2026-09-05',
          state: 'pending',
          purchased_at: '2026-09-05T09:01:00Z',
          revision: 1,
        };
        return ok(api.purchase, 201);
      }),
      PATCH: vi.fn(),
      PUT: vi.fn(),
      DELETE: vi.fn(),
    },
  };
});

beforeEach(() => {
  api.trip = undefined;
  api.purchase = undefined;
  api.requests.length = 0;
});

describe('TripPage queries', () => {
  it('records a real purchase and keeps the row ticked after invalidation', async () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <TripPage />
      </QueryClientProvider>,
    );

    const checkbox = await screen.findByRole('checkbox', { name: 'Bought Whole Milk' });
    await userEvent.setup().click(checkbox);

    await waitFor(() =>
      expect(screen.getByRole('checkbox', { name: 'Bought Whole Milk' })).toBeChecked(),
    );
    expect(api.requests).toContain('POST /api/v1/purchases');
    expect(api.requests).toContain('POST /api/v1/shopping/opportunities/{date}/start');
  });
});
