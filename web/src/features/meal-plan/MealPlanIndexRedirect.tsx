import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState, Loading } from '../../components/States';
import { startOfWeekIso, todayIso } from './date';
import { defaultDayFor } from './MealPlanPage';

export function MealPlanIndexRedirect({ workspace }: { workspace: 'today' | 'planner' }) {
  const { principal } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    if (!principal?.member_id) return;
    const weekStart = startOfWeekIso(todayIso());
    void navigate({
      to: workspace === 'today' ? '/food-log/$weekStart/$day' : '/planner/$weekStart/$day',
      params: { weekStart, day: defaultDayFor(weekStart) },
      replace: true,
    });
  }, [navigate, principal?.member_id, workspace]);

  if (!principal?.member_id) {
    return (
      <EmptyState
        title={workspace === 'today' ? 'Food log unavailable' : 'Meal planner unavailable'}
        description="Your account is not linked to an active household member."
      />
    );
  }
  return <Loading label={workspace === 'today' ? 'Loading food log' : 'Loading meal planner'} />;
}
