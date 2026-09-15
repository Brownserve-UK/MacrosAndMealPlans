import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState } from '../../components/States';
import { HouseholdLens } from './HouseholdLens';
import { MineLens } from './MineLens';
import type { Lens } from './PlannerLens';
import { usePlannerLens } from './usePlannerLens';

export function PlannerPage({
  weekStart,
  day,
  requestedLens,
}: {
  weekStart: string;
  day: string;
  requestedLens?: Lens;
}) {
  const { principal } = useAuth();
  const navigate = useNavigate();
  const { remember } = usePlannerLens();
  const canUseMine = Boolean(principal?.member_id);
  const canUseHousehold = principal?.permissions?.includes('household:write') ?? false;
  const lens: Lens = requestedLens === 'household' && canUseHousehold
    ? 'household'
    : requestedLens === 'mine' && canUseMine
      ? 'mine'
      : canUseMine
        ? 'mine'
        : 'household';

  useEffect(() => {
    if (!requestedLens || requestedLens === lens) return;
    void navigate({
      to: '/planner/$weekStart/$day',
      params: { weekStart, day },
      search: { lens },
      replace: true,
    });
  }, [day, lens, navigate, requestedLens, weekStart]);

  if (!canUseMine && !canUseHousehold) {
    return (
      <EmptyState
        title="Meal planner unavailable"
        description="Your account is not linked to an active household member."
      />
    );
  }

  function changeLens(next: Lens) {
    remember(next);
    // 2026-09-15 - SB: We regressed the Planner nav by sending the Household lens to a Household route, so we'll keep both lenses on the Planner route.
    void navigate({
      to: '/planner/$weekStart/$day',
      params: { weekStart, day },
      search: { lens: next },
      replace: true,
    });
  }

  const shared = {
    weekStart,
    day,
    showLens: canUseMine && canUseHousehold,
    onLensChange: changeLens,
  };
  return lens === 'household'
    ? <HouseholdLens {...shared} enabled />
    : <MineLens {...shared} enabled />;
}
