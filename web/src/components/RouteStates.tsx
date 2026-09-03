import Button from '@mui/material/Button';
import { Link, useRouter } from '@tanstack/react-router';
import { EmptyState, ErrorState } from './States';

export function RouteNotFound() {
  return (
    <EmptyState
      title="Page not found"
      description="That page does not exist or has moved."
      action={
        <Button component={Link} to="/food-log" variant="contained">
          Go to food log
        </Button>
      }
    />
  );
}

export function RouteError({ error }: { error: unknown }) {
  const router = useRouter();
  return <ErrorState error={error} onRetry={() => void router.invalidate()} />;
}
