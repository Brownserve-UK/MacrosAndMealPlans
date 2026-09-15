import { createMemoryHistory, createRouter } from '@tanstack/react-router';
import { describe, expect, it } from 'vitest';
import { routeTree } from './router';

describe('planner redirects', () => {
  it('redirects a household planner day deep link to the Household lens', async () => {
    const history = createMemoryHistory({
      initialEntries: ['/household/planner/2026-09-14/2026-09-15'],
    });
    const testRouter = createRouter({ routeTree, history });
    await testRouter.load();
    expect(history.location.href).toBe('/planner/2026-09-14/2026-09-15?lens=household');
  });
});
