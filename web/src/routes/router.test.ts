import { createMemoryHistory, createRouter } from '@tanstack/react-router';
import { describe, expect, it } from 'vitest';
import { routeTree } from './router';

async function landingFor(path: string): Promise<string> {
  const history = createMemoryHistory({ initialEntries: [path] });
  const testRouter = createRouter({ routeTree, history });
  await testRouter.load();
  return history.location.href;
}

describe('planner redirects', () => {
  it('redirects an old planner day deep link to the week', async () => {
    expect(await landingFor('/planner/2026-09-14/2026-09-15')).toBe('/planner/2026-09-14');
  });
});

describe('my food redirects', () => {
  it('sends the root to My food', async () => {
    expect(await landingFor('/')).toBe('/my-food');
  });

  it('keeps old food log links working', async () => {
    expect(await landingFor('/food-log')).toBe('/my-food');
    expect(await landingFor('/food-log/2026-09-14/2026-09-15')).toBe('/my-food/2026-09-14/2026-09-15');
  });

  it('picks a day for a bare week link', async () => {
    expect(await landingFor('/food-log/2026-09-14')).toMatch(/^\/my-food\/2026-09-14\/2026-09-\d\d$/);
  });
});
