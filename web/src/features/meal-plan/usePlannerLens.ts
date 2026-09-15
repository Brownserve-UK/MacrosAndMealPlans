import { useCallback } from 'react';
import type { Lens } from './PlannerLens';

const STORAGE_KEY = 'mmp.planner-lens';

export function usePlannerLens() {
  const read = useCallback((canUseHousehold: boolean): Lens => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      return stored === 'household' && canUseHousehold ? 'household' : 'mine';
    } catch {
      return 'mine';
    }
  }, []);

  const remember = useCallback((lens: Lens) => {
    try {
      localStorage.setItem(STORAGE_KEY, lens);
    } catch {
      return;
    }
  }, []);

  return { read, remember };
}
