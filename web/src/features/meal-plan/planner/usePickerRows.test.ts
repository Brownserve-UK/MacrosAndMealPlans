import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { StockItem } from '../../../api/client';
import { leftoversRow, useLeftoverDishes } from './usePickerRows';

const mocks = vi.hoisted(() => ({ items: [] as StockItem[] }));
vi.mock('../../../api/queries', () => ({
  useStock: () => ({ data: { items: mocks.items } }),
}));

function portion(servings: number, date: string | null, storage: StockItem['storage_location'] = 'chilled', overrides: Partial<StockItem> = {}): StockItem {
  return {
    id: crypto.randomUUID(),
    subject_kind: 'prepared_portion',
    prepared_recipe_id: 'curry',
    prepared_batch_name: 'Curry',
    level: { mode: 'exact', quantity: { amount: servings, unit: 'serving' } },
    tracking_mode: 'exact',
    storage_location: storage,
    usability_deadline: date ? { date } : null,
    revision: 1,
    created_at: '2026-09-20T09:00:00Z',
    updated_at: '2026-09-20T09:00:00Z',
    ...overrides,
  };
}

describe('leftovers suggestions', () => {
  beforeEach(() => { mocks.items = []; });

  it('filters portions before totalling and recalculates when the planned date changes', () => {
    mocks.items = [
      portion(10, '2026-09-21'),
      portion(2, '2026-09-22'),
      portion(5, '2026-12-19', 'frozen'),
      portion(20, null, 'ambient'),
      portion(1, '2026-09-22', 'ambient'),
    ];
    const { result, rerender } = renderHook(({ date }) => useLeftoverDishes(date), { initialProps: { date: '2026-09-22' } });
    expect(result.current).toHaveLength(1);
    expect(result.current[0]?.servings).toBe(8);
    expect(leftoversRow(result.current[0]!).caption).toBe('Curry, 8 servings (2 chilled · 5 frozen · 1 ambient)');
    rerender({ date: '2026-11-20' });
    expect(result.current[0]?.servings).toBe(5);
    expect(leftoversRow(result.current[0]!).caption).toBe('Curry, 5 servings (5 frozen)');
    rerender({ date: '2026-12-20' });
    expect(result.current).toEqual([]);
  });

  it('excludes undated, archived, empty and unmeasured portions', () => {
    mocks.items = [
      portion(1, null),
      portion(1, '2026-09-22', 'frozen', { archived_at: '2026-09-20T10:00:00Z' }),
      portion(0, '2026-09-22'),
      portion(1, '2026-09-22', 'chilled', { level: { mode: 'not_tracked' } }),
      portion(1, '2026-09-22', 'chilled', { subject_kind: 'product' }),
    ];
    const { result } = renderHook(() => useLeftoverDishes('2026-09-22'));
    expect(result.current).toEqual([]);
  });
});
