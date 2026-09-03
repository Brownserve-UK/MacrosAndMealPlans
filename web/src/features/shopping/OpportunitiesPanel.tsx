import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { ShoppingOpportunity } from '../../api/client';
import { useSkipShoppingOpportunity } from '../../api/queries';
import { formatDayLabel } from '../meal-plan/date';

export function OpportunitiesPanel({
  opportunities,
}: {
  opportunities: ShoppingOpportunity[];
}) {
  const [open, setOpen] = useState(false);
  const skip = useSkipShoppingOpportunity();

  const next = opportunities[0];
  if (!next) return null;

  const upcoming = opportunities.slice(0, 5);

  return (
    <Paper variant="outlined" sx={{ p: 2, mb: 2.5 }}>
      <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="body2" color="text.secondary">
          Next shop {formatDayLabel(next.date)}
        </Typography>
        <Button size="small" onClick={() => setOpen(!open)}>
          {open ? 'Hide' : 'Upcoming shops'}
        </Button>
      </Stack>

      {open && (
        <Stack spacing={1} sx={{ mt: 1.5 }}>
          {upcoming.map((opportunity) => (
            <Stack
              key={opportunity.date}
              direction="row"
              spacing={1}
              sx={{ alignItems: 'center' }}
            >
              <Typography variant="body2" sx={{ flex: 1 }}>
                {formatDayLabel(opportunity.date)}
              </Typography>
              {opportunity.state === 'moved' && <Chip size="small" label="Moved" />}
              {opportunity.state === 'one_off' && <Chip size="small" label="Extra" />}
              <Button
                size="small"
                disabled={skip.isPending}
                onClick={() => skip.mutate(opportunity.generated_for ?? opportunity.date)}
              >
                Skip
              </Button>
            </Stack>
          ))}
        </Stack>
      )}
    </Paper>
  );
}
