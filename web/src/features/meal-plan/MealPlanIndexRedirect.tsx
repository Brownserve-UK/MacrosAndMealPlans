import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { defaultDayFor, startOfWeekIso, todayIso } from './date';

export function MealPlanIndexRedirect() {
  const { principal } = useAuth();
  const navigate = useNavigate();
  const timeZone = useHouseholdTimeZone();

  useEffect(() => {
    if (!principal?.member_id) return;
    const weekStart = startOfWeekIso(todayIso(timeZone));
    const day = defaultDayFor(weekStart, timeZone);
    void navigate({
      to: '/my-food/$weekStart/$day',
      params: { weekStart, day },
      replace: true,
    });
  }, [navigate, principal, timeZone]);

  if (!principal?.member_id) {
    return (
      <EmptyState
        title="My food unavailable"
        description="Your account is not linked to an active household member."
      />
    );
  }
  return <Loading label="Loading My food" />;
}
