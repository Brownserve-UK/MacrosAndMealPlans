import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState, Loading } from '../../components/States';
import { startOfWeekIso, todayIso } from './date';
import { defaultDayFor } from './MealPlanPage';

export function MealPlanIndexRedirect() {
  const { principal } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    if (!principal?.member_id) return;
    const weekStart = startOfWeekIso(todayIso());
    void navigate({
      to: '/meal-plan/$weekStart/$day',
      params: { weekStart, day: defaultDayFor(weekStart) },
      replace: true,
    });
  }, [navigate, principal?.member_id]);

  if (!principal?.member_id) {
    return (
      <EmptyState
        title="No meal plan yet"
        description="Your account is not linked to an active household member."
      />
    );
  }
  return <Loading label="Loading meal plan" />;
}
