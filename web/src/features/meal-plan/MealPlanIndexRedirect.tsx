import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { defaultDayFor, startOfWeekIso, todayIso } from './date';
import { usePlannerLens } from './usePlannerLens';

type IndexTarget = '/food-log' | '/planner';

const LABELS: Record<IndexTarget, { loading: string; unavailable: string }> = {
  '/food-log': { loading: 'Loading food log', unavailable: 'Food log unavailable' },
  '/planner': { loading: 'Loading planner', unavailable: 'Meal planner unavailable' },
};

export function MealPlanIndexRedirect({ to, lens }: { to: IndexTarget; lens?: 'mine' | 'household' }) {
  const { principal } = useAuth();
  const navigate = useNavigate();
  const { read: readPlannerLens } = usePlannerLens();
  const canUseHousehold = principal?.permissions?.includes('household:write') ?? false;
  const needsMember = to === '/food-log' || (to === '/planner' && !principal?.member_id && !canUseHousehold);
  const timeZone = useHouseholdTimeZone();

  useEffect(() => {
    if (needsMember && !principal?.member_id) return;
    if (!principal) return;
    const weekStart = startOfWeekIso(todayIso(timeZone));
    const day = defaultDayFor(weekStart, timeZone);
    if (to === '/planner') {
      const defaultLens = principal.member_id ? readPlannerLens(canUseHousehold) : 'household';
      const targetLens = lens ?? defaultLens;
      void navigate({
        to: '/planner/$weekStart/$day',
        params: { weekStart, day },
        search: { lens: targetLens },
        replace: true,
      });
    } else {
      void navigate({
        to: '/food-log/$weekStart/$day',
        params: { weekStart, day },
        replace: true,
      });
    }
  }, [canUseHousehold, lens, navigate, needsMember, principal, readPlannerLens, timeZone, to]);

  if (needsMember && !principal?.member_id) {
    return (
      <EmptyState
        title={LABELS[to].unavailable}
        description="Your account is not linked to an active household member."
      />
    );
  }
  return <Loading label={LABELS[to].loading} />;
}
