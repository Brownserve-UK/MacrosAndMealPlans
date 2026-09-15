import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePlannerLens } from './usePlannerLens';

const values = new Map<string, string>();
const storage = {
  getItem: vi.fn((key: string) => values.get(key) ?? null),
  setItem: vi.fn((key: string, value: string) => values.set(key, value)),
};

describe('usePlannerLens', () => {
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });

  afterEach(() => {
    values.clear();
    storage.getItem.mockClear();
    storage.setItem.mockClear();
    vi.restoreAllMocks();
  });

  it('remembers an available household lens', () => {
    const { result } = renderHook(() => usePlannerLens());
    result.current.remember('household');
    expect(result.current.read(true)).toBe('household');
  });

  it('clamps a remembered household lens without permission', () => {
    localStorage.setItem('mmp.planner-lens', 'household');
    const { result } = renderHook(() => usePlannerLens());
    expect(result.current.read(false)).toBe('mine');
  });

  it('falls back when storage cannot be read', () => {
    storage.getItem.mockImplementationOnce(() => {
      throw new Error('storage unavailable');
    });
    const { result } = renderHook(() => usePlannerLens());
    expect(result.current.read(true)).toBe('mine');
  });
});
