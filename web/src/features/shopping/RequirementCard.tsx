import ScheduleIcon from '@mui/icons-material/ScheduleOutlined';
import WarningIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Checkbox from '@mui/material/Checkbox';
import Stack from '@mui/material/Stack';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import type { ShoppingRequirement } from '../../api/client';
import { formatQuantity } from '../stock/SpokenFor';

export function needsToKeep(
  requirement: ShoppingRequirement,
  nextShopAfter?: string | null,
): boolean {
  if (!requirement.use_by_at_least || !nextShopAfter) return false;
  return requirement.use_by_at_least >= nextShopAfter;
}

export function RequirementCard({
  requirement,
  nextShopAfter,
  bought,
  onToggle,
  onOpen,
}: {
  requirement: ShoppingRequirement;
  nextShopAfter?: string | null;
  bought?: boolean;
  onToggle?: (next: boolean) => void;
  onOpen: () => void;
}) {
  const sooner = requirement.assignment.kind === 'needs_earlier_opportunity';
  const keeps = needsToKeep(requirement, nextShopAfter);

  return (
    <Stack
      direction="row"
      spacing={1}
      sx={{
        alignItems: 'center',
        px: onToggle ? 1 : 2,
        py: 1.25,
        opacity: bought ? 0.5 : 1,
      }}
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
          gap: 1,
          background: 'none',
          border: 0,
          font: 'inherit',
          color: 'inherit',
          textAlign: 'left',
          cursor: 'pointer',
          p: 0,
        }}
      >
        <Typography
          variant="body1"
          sx={{ fontWeight: 500, textDecoration: bought ? 'line-through' : 'none' }}
        >
          {requirement.name}
        </Typography>
        {requirement.quantity ? (
          <Typography variant="body2" color="text.secondary">
            {formatQuantity(requirement.quantity)}
          </Typography>
        ) : null}
      </Box>

      {sooner ? (
        <Tooltip title="Needed before your next shop">
          <WarningIcon fontSize="small" color="warning" />
        </Tooltip>
      ) : null}
      {keeps ? (
        <Tooltip title="Has to last until the shop after this one">
          <ScheduleIcon fontSize="small" sx={{ color: 'text.disabled' }} />
        </Tooltip>
      ) : null}
    </Stack>
  );
}
