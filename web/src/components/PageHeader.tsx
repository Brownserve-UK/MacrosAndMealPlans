import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import { SearchField } from './SearchField';

export function PageHeader({
  title,
  subtitle,
  actions,
  search,
  back,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  back?: ReactNode;
  search?: { value: string; onChange: (next: string) => void; placeholder: string };
}) {
  return (
    <Stack spacing={3} sx={{ mb: 3 }}>
      {back}
      <Stack
        direction={{ xs: 'column', sm: 'row' }}
        spacing={2}
        sx={{
          justifyContent: 'space-between',
          alignItems: { xs: 'stretch', sm: 'flex-end' },
        }}
      >
        <Stack spacing={0.75} sx={{ minWidth: 0 }}>
          <Typography variant="h1">{title}</Typography>
          {subtitle ? (
            <Typography variant="body2" color="text.secondary" sx={{ maxWidth: 560 }}>
              {subtitle}
            </Typography>
          ) : null}
        </Stack>
        {actions ? (
          <Stack direction="row" spacing={1} sx={{ flexShrink: 0 }}>
            {actions}
          </Stack>
        ) : null}
      </Stack>

      {search ? (
        <SearchField
          value={search.value}
          onChange={search.onChange}
          placeholder={search.placeholder}
        />
      ) : null}
    </Stack>
  );
}
