import AddIcon from '@mui/icons-material/AddOutlined';
import ClockIcon from '@mui/icons-material/AccessTimeOutlined';
import MoreIcon from '@mui/icons-material/MoreHorizOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { MealPlanEntry } from '../../api/client';
import { formatAmount } from './format';

function occurrenceSort(a: MealPlanEntry, b: MealPlanEntry) {
  if (!a.planned_time && !b.planned_time) return 0;
  if (!a.planned_time) return 1;
  if (!b.planned_time) return -1;
  return a.planned_time.localeCompare(b.planned_time);
}

function statusChip(entry: MealPlanEntry) {
  if (entry.status === 'eaten') return <Chip size="small" color="success" label="Eaten" />;
  if (entry.status === 'not_eaten') return <Chip size="small" label="Not eaten" />;
  if (entry.status === 'partially_resolved') return <Chip size="small" label="Partly recorded" />;
  return null;
}

function SnackOccurrence({
  entry,
  editable,
  onAddFood,
  onEdit,
  onDelete,
}: {
  entry: MealPlanEntry;
  editable: boolean;
  onAddFood: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const close = () => setAnchor(null);

  return (
    <Box role="group" aria-label={entry.planned_time ? `Snack at ${entry.planned_time}` : 'Untimed snack'}>
      <Stack>
        {entry.components.map((component) => (
          <Stack
            key={component.id}
            direction="row"
            spacing={2}
            sx={{ justifyContent: 'space-between', px: { xs: 2, sm: 2.5 }, pt: 1.5, pb: 0.75 }}
          >
            <Typography>{component.item_name}</Typography>
            <Typography color="text.secondary" sx={{ whiteSpace: 'nowrap' }}>
              {formatAmount(component.amount)}
            </Typography>
          </Stack>
        ))}
      </Stack>
      <Stack
        direction="row"
        spacing={1}
        sx={{ alignItems: 'center', justifyContent: 'space-between', px: { xs: 2, sm: 2.5 }, pb: 1, pt: 0.5 }}
      >
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          {entry.planned_time ? (
            <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', color: 'text.secondary' }}>
              <ClockIcon sx={{ fontSize: 15 }} />
              <Typography variant="caption" className="numeral">{entry.planned_time}</Typography>
            </Stack>
          ) : (
            <Typography variant="caption" color="text.secondary">No set time</Typography>
          )}
          {statusChip(entry)}
        </Stack>
        {editable ? (
          <>
            <IconButton size="small" aria-label={`Snack${entry.planned_time ? ` at ${entry.planned_time}` : ''} actions`} onClick={(event) => setAnchor(event.currentTarget)}>
              <MoreIcon fontSize="small" />
            </IconButton>
            <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={close}>
              <MenuItem onClick={() => { close(); onAddFood(); }}>Add food</MenuItem>
              <MenuItem onClick={() => { close(); onEdit(); }}>Edit snack</MenuItem>
              <MenuItem onClick={() => { close(); onDelete(); }} sx={{ color: 'error.main' }}>Delete</MenuItem>
            </Menu>
          </>
        ) : null}
      </Stack>
    </Box>
  );
}

export function SnackSection({
  entries,
  memberId,
  canPlan,
  onAddSnack,
  onAddFood,
  onEdit,
  onDelete,
}: {
  entries: MealPlanEntry[];
  memberId: string | null | undefined;
  canPlan: boolean;
  onAddSnack: () => void;
  onAddFood: (entry: MealPlanEntry) => void;
  onEdit: (entry: MealPlanEntry) => void;
  onDelete: (entry: MealPlanEntry) => void;
}) {
  const occurrences = [...entries].sort(occurrenceSort);

  if (occurrences.length === 0) {
    return canPlan ? (
      <Button
        fullWidth
        startIcon={<AddIcon />}
        onClick={onAddSnack}
        sx={{ justifyContent: 'center', py: 1.5, borderRadius: 1.5, border: '1px dashed', borderColor: 'divider', color: 'primary.main', '&:hover': { borderColor: 'primary.main', bgcolor: 'action.hover' } }}
      >
        Add snack
      </Button>
    ) : (
      <Typography variant="body2" color="text.secondary">No snacks planned</Typography>
    );
  }

  return (
    <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
      <Stack divider={<Divider flexItem />}>
        {occurrences.map((entry) => {
          const editable = canPlan && entry.scope === 'member'
            && (entry.member_id == null || entry.member_id === memberId);
          return (
            <SnackOccurrence
              key={entry.id}
              entry={entry}
              editable={editable}
              onAddFood={() => onAddFood(entry)}
              onEdit={() => onEdit(entry)}
              onDelete={() => onDelete(entry)}
            />
          );
        })}
      </Stack>
      {canPlan ? (
        <>
          <Divider />
          <Box sx={{ px: { xs: 1.5, sm: 2 }, py: 1 }}>
            <Button size="small" startIcon={<AddIcon />} onClick={onAddSnack}>
              Add snack
            </Button>
          </Box>
        </>
      ) : null}
    </Paper>
  );
}
