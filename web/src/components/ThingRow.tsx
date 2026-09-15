import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import type { Concept } from './ConceptIcon';
import { IconTile, type Tone } from './IconTile';

export function ThingRow({
  concept,
  tone,
  title,
  caption,
  chip,
  action,
}: {
  concept: Concept;
  tone?: Tone;
  title: ReactNode;
  caption?: ReactNode;
  chip?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <Paper sx={{ display: 'flex', alignItems: 'center', gap: 2, px: 2.25, py: 1.75 }}>
      <IconTile concept={concept} tone={tone} />

      <Stack sx={{ minWidth: 0, flexGrow: 1 }}>
        <Typography
          variant="subtitle1"
          sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
        >
          {title}
        </Typography>
        {caption ? (
          <Typography variant="caption" color="text.secondary">
            {caption}
          </Typography>
        ) : null}
      </Stack>

      {chip}
      {action ? <Box sx={{ flexShrink: 0 }}>{action}</Box> : null}
    </Paper>
  );
}
