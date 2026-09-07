import Box from '@mui/material/Box';
import Checkbox from '@mui/material/Checkbox';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ShoppingRequirement } from '../../api/client';
import { formatDayLabel } from '../meal-plan/date';
import { formatQuantity } from '../stock/SpokenFor';

export function amountOf(requirement: ShoppingRequirement): string | null {
  if (!requirement.quantity) return null;
  return formatQuantity(requirement.quantity);
}

export function RequirementCard({
  requirement,
  bought,
  onToggle,
  onOpen,
}: {
  requirement: ShoppingRequirement;
  bought?: boolean;
  onToggle?: (next: boolean) => void;
  onOpen: () => void;
}) {
  const amount = amountOf(requirement);
  const keepUntil = requirement.use_by_at_least;

  return (
    <Stack
      direction="row"
      spacing={1.5}
      sx={{ alignItems: 'center', px: onToggle ? 1 : 2, py: 0.5, borderTop: 1, borderColor: 'divider' }}
    >
      {onToggle ? (
        <Checkbox
          checked={Boolean(bought)}
          onChange={(_, checked) => onToggle(checked)}
          slotProps={{ input: { 'aria-label': `Bought ${requirement.name}` } }}
        />
      ) : null}

      <Box
        component="button"
        type="button"
        onClick={onOpen}
        sx={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 1.5,
          background: 'none',
          border: 0,
          font: 'inherit',
          color: 'inherit',
          textAlign: 'left',
          cursor: 'pointer',
          py: 1,
          px: 0,
        }}
      >
        <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
          <Typography
            variant="body1"
            sx={{
              fontWeight: 500,
              color: bought ? 'text.disabled' : 'text.primary',
              textDecoration: bought ? 'line-through' : 'none',
            }}
          >
            {requirement.name}
          </Typography>
          {keepUntil ? (
            <Typography variant="caption" color="text.secondary" className="numeral">
              Use by at least: {formatDayLabel(keepUntil)}
            </Typography>
          ) : null}
        </Stack>

        {amount ? (
          <Typography
            variant="body1"
            className="numeral"
            sx={{ fontWeight: 500, color: bought ? 'text.disabled' : 'text.primary' }}
          >
            {amount}
          </Typography>
        ) : null}
      </Box>
    </Stack>
  );
}
