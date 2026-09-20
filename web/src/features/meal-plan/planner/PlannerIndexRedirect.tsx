import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { Loading } from '../../../components/States';
import { useHouseholdTimeZone } from '../../../hooks/useHouseholdTimeZone';
import { startOfWeekIso, todayIso } from '../date';

export function PlannerIndexRedirect() {
  const navigate = useNavigate();
  const timeZone = useHouseholdTimeZone();

  useEffect(() => {
    void navigate({
      to: '/planner/$weekStart',
      params: { weekStart: startOfWeekIso(todayIso(timeZone)) },
      replace: true,
    });
  }, [navigate, timeZone]);

  return <Loading label="Loading planner" />;
}
