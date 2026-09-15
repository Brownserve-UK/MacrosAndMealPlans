import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { ShoppingRequirement } from '../../api/client';
import { AddShopDialog } from './AddShopDialog';
import { formatDayLabel } from '../meal-plan/date';

function when(iso: string): string {
  const label = formatDayLabel(iso);
  return label === 'Today' || label === 'Tomorrow' ? label.toLowerCase() : `by ${label}`;
}

export function GapCard({ requirement }: { requirement: ShoppingRequirement }) {
  const [adding, setAdding] = useState(false);
  const by = requirement.required_by;

  return (
    <>
      <Box
        sx={{
          border: '1px dashed',
          borderColor: 'divider',
          borderRadius: 3.5,
          p: 2.5,
        }}
      >
        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={2}
          sx={{ alignItems: { sm: 'center' } }}
        >
          <Box sx={{ flexGrow: 1, minWidth: 0 }}>
            <Typography variant="h2" color="text.secondary">
              No shop in time
            </Typography>
            <Typography variant="body2" color="warning.main" className="numeral">
              {by ? `${requirement.name} is needed ${when(by)}` : `${requirement.name} has nowhere to be bought`}
            </Typography>
          </Box>
          <Button variant="outlined" onClick={() => setAdding(true)} sx={{ flexShrink: 0 }}>
            Add a shop
          </Button>
        </Stack>
      </Box>

      <AddShopDialog open={adding} onClose={() => setAdding(false)} />
    </>
  );
}
