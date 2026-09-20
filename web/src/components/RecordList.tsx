import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import { InitialsAvatar } from './InitialsAvatar';

export function RecordRow({
  name,
  detail,
  trailing,
  muted,
}: {
  name: string;
  detail: ReactNode;
  trailing?: ReactNode;
  muted?: boolean;
}) {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        px: 2.5,
        py: 1.75,
        transition: 'background-color 120ms ease',
        opacity: muted ? 0.55 : 1,
        '&:hover': { backgroundColor: 'action.hover' },
        '&:hover .chevron': { opacity: 1 },
      }}
    >
      <InitialsAvatar name={name} />

      <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
        <Typography
          variant="subtitle1"
          sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
        >
          {name}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {detail}
        </Typography>
      </Stack>

      {trailing}
      <ChevronRightIcon
        className="chevron"
        fontSize="small"
        sx={{ color: 'text.disabled', opacity: 0, transition: 'opacity 120ms ease' }}
      />
    </Box>
  );
}

export function RecordListShell({ children }: { children: ReactNode }) {
  return (
    <Paper
      sx={{
        overflow: 'hidden',
        '& > a': { display: 'block', textDecoration: 'none', color: 'inherit' },
        '& > a + a': { borderTop: '1px solid', borderColor: 'divider' },
      }}
    >
      {children}
    </Paper>
  );
}
