import { Link } from '@tanstack/react-router';
import { useAuth } from '../../auth/AuthProvider';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { EmptyState } from '../../components/States';

const SECTIONS = [
  {
    to: '/administration/accounts' as const,
    label: 'Accounts',
    detail: 'Sign-in and roles',
    needs: 'account:admin',
  },
  {
    to: '/administration/meal-times' as const,
    label: 'Meal times',
    detail: 'Default times for planned meals',
    needs: 'household:write',
  },
] as const;

export function AdministrationPage() {
  const { principal } = useAuth();
  const sections = SECTIONS.filter((s) => principal?.permissions.includes(s.needs) ?? false);

  return (
    <>
      <PageHeader title="Administration" />

      {sections.length === 0 ? (
        <EmptyState title="Not available" description="This needs admin access." />
      ) : (
        <RecordListShell>
          {sections.map((section) => (
            <Link key={section.to} to={section.to}>
              <RecordRow name={section.label} detail={section.detail} />
            </Link>
          ))}
        </RecordListShell>
      )}
    </>
  );
}
