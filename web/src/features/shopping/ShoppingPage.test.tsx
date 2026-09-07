import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { shoppingList } from './fixtures';
import { ShoppingPage } from './ShoppingPage';

beforeAll(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.setSystemTime(new Date('2026-09-01T09:00:00Z'));
});

afterAll(() => {
  vi.useRealTimers();
});

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => children,
}));

const putAway: unknown[] = [];

vi.mock('../../api/queries', () => ({
  useShoppingList: () => ({ isLoading: false, isError: false, data: shoppingList }),
  usePendingPutAway: () => ({ data: putAway }),
  useMoveShoppingOpportunity: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useSkipShoppingOpportunity: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useAddShoppingOpportunity: () => ({ isPending: false, mutateAsync: vi.fn() }),
}));

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ShoppingPage />
    </QueryClientProvider>,
  );
}

describe('ShoppingPage', () => {
  it('is a stack of trips, not a list of things to buy', () => {
    renderPage();

    expect(screen.getByText('This Saturday')).toBeInTheDocument();
    expect(screen.getByText(/Sat 12 Sept?/)).toBeInTheDocument();
    expect(screen.queryByText('Whole Milk')).not.toBeInTheDocument();
    expect(screen.queryByText('Butter')).not.toBeInTheDocument();
  });

  it('counts what each trip is for', () => {
    renderPage();

    expect(screen.getByText(/3 things/)).toBeInTheDocument();
    expect(screen.getByText(/8 things/)).toBeInTheDocument();
  });

  it('leaves out a later trip with nothing to buy', () => {
    renderPage();

    expect(screen.queryByText(/Sat 19 Sept?/)).not.toBeInTheDocument();
  });

  it('offers a shop for anything no trip can reach in time', () => {
    renderPage();

    expect(screen.getByText('No shop in time')).toBeInTheDocument();
    expect(screen.getByText(/Plain Flour is needed/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add a shop' })).toBeInTheDocument();
  });

  it('says none of the internal vocabulary out loud', () => {
    renderPage();

    for (const leak of [/unassigned/i, /assumption/i, /incompatible/i, /pending/i]) {
      expect(screen.queryByText(leak)).not.toBeInTheDocument();
    }
  });

  it('keeps put away out of the way when there is nothing waiting', () => {
    renderPage();

    expect(screen.queryByText('Put the shopping away')).not.toBeInTheDocument();
  });
});

describe('ShoppingPage with shopping to unpack', () => {
  it('leads with put away', () => {
    putAway.push({ id: 'p1' }, { id: 'p2' }, { id: 'p3' });
    renderPage();

    const heading = screen.getByText('Put the shopping away');
    expect(heading).toBeInTheDocument();
    expect(within(heading.parentElement!).getByText('3 things, still in the bags'))
      .toBeInTheDocument();
    putAway.length = 0;
  });
});
