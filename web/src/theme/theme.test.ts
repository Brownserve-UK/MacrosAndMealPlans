import { describe, expect, it } from 'vitest';
import { theme } from './theme';

type Override = (args: { theme: typeof theme }) => Record<string, unknown>;

function resolve(component: 'MuiAppBar' | 'MuiDrawer' | 'MuiPaper', slot: string) {
  const overrides = theme.components?.[component]?.styleOverrides as
    | Record<string, Override>
    | undefined;
  const fn = overrides?.[slot];
  if (typeof fn !== 'function') throw new Error(`${component}.${slot} override is missing`);
  return fn({ theme });
}

describe('theme colour scheme', () => {
  it('paints the app bar with a scheme aware variable', () => {
    const style = resolve('MuiAppBar', 'root');
    expect(String(style.backgroundColor)).toMatch(/^var\(--/);
  });

  it('paints the drawer with a scheme aware variable', () => {
    const style = resolve('MuiDrawer', 'paper');
    expect(String(style.backgroundColor)).toMatch(/^var\(--/);
  });

  it('uses scheme aware borders', () => {
    for (const [component, slot] of [
      ['MuiAppBar', 'root'],
      ['MuiDrawer', 'paper'],
      ['MuiPaper', 'root'],
    ] as const) {
      const style = resolve(component, slot);
      const borders = [style.border, style.borderBottom, style.borderRight]
        .map(String)
        .filter((value) => value !== 'undefined' && value !== 'none');
      expect(borders.join(' '), `${component}.${slot}`).toMatch(/var\(--/);
    }
  });

  it('defines both colour schemes', () => {
    const schemes = (theme as unknown as { colorSchemes?: Record<string, unknown> }).colorSchemes;
    expect(schemes?.light).toBeDefined();
    expect(schemes?.dark).toBeDefined();
  });
});
