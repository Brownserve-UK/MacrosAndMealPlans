import type { ReactNode } from 'react';
import { useAuth } from '../auth/AuthProvider';
import { EmptyState } from './States';

export function RequirePermission({
  permission,
  children,
}: {
  permission: string;
  children: ReactNode;
}) {
  const { principal } = useAuth();
  if (!principal) return null;
  if (!principal.permissions.includes(permission)) {
    return (
      <EmptyState
        title="Not available"
        description="You do not have permission to open this page."
      />
    );
  }
  return <>{children}</>;
}
