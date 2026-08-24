import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { useAuth } from '../../auth/AuthProvider';
import { EmptyState, Loading } from '../../components/States';
import { todayIso } from './date';

export function DiaryIndexRedirect() {
  const { principal } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    if (principal?.member_id) {
      void navigate({
        to: '/diary/$memberId/$date',
        params: { memberId: principal.member_id, date: todayIso() },
        replace: true,
      });
    }
  }, [principal, navigate]);

  if (!principal?.member_id) {
    return (
      <EmptyState
        title="No diary yet"
        description="Your account is not linked to a household member, so there is no diary to show."
      />
    );
  }
  return <Loading label="Loading diary" />;
}
