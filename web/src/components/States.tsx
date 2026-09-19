import Alert from '@mui/material/Alert';
import AlertTitle from '@mui/material/AlertTitle';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import CircularProgress from '@mui/material/CircularProgress';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import { ApiError } from '../api/client';

export function Loading({ label = 'Loading' }: { label?: string }) {
  return (
    <Stack spacing={2} sx={{ py: 8, alignItems: 'center', justifyContent: 'center' }}>
      <CircularProgress size={28} />
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
    </Stack>
  );
}

export function ErrorState({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const problem = error instanceof ApiError ? error.problem : null;
  const title = problem?.title ?? 'Something went wrong';
  const detail =
    problem?.detail ?? (error instanceof Error ? error.message : 'An unexpected error occurred.');

  return (
    <Alert
      severity="error"
      sx={{ my: 2 }}
      action={
        onRetry ? (
          <Button color="inherit" size="small" onClick={onRetry}>
            Retry
          </Button>
        ) : undefined
      }
    >
      <AlertTitle>{title}</AlertTitle>
      {detail}
    </Alert>
  );
}

export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <Box
      sx={{
        py: 8,
        px: 3,
        textAlign: 'center',
        border: '1px dashed',
        borderColor: 'divider',
        borderRadius: 2,
      }}
    >
      <Typography variant="h3" gutterBottom>
        {title}
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ maxWidth: 460, mx: 'auto', mb: 2 }}>
        {description}
      </Typography>
      {action}
    </Box>
  );
}
