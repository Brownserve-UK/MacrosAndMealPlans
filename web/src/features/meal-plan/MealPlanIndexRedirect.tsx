import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useMealPlanMembers } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { startOfWeekIso, todayIso } from '../diary/date';

export function MealPlanIndexRedirect() {
  const { principal } = useAuth();
  const members = useMealPlanMembers();
  const navigate = useNavigate();

  useEffect(() => {
    if (!members.data) return;
    const member =
      members.data.find((candidate) => candidate.id === principal?.member_id) ?? members.data[0];
    if (member) {
      void navigate({
        to: '/meal-plan/$memberId/$weekStart',
        params: { memberId: member.id, weekStart: startOfWeekIso(todayIso()) },
        replace: true,
      });
    }
  }, [members.data, navigate, principal?.member_id]);

  if (members.isError) {
    return <ErrorState error={members.error} onRetry={() => members.refetch()} />;
  }
  if (members.isLoading) return <Loading label="Loading meal plan" />;
  if (members.data?.length === 0) {
    return (
      <EmptyState
        title="No meal plan yet"
        description="Your account is not linked to an active household member."
      />
    );
  }
  return <Loading label="Loading meal plan" />;
}
