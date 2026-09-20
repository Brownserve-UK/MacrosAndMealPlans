import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { MealSlot } from '../../../api/client';
import { FormDialog } from '../../../components/FormDialog';
import { addDays, parseIsoDate } from '../date';
import { labelForSlot } from '../slots';
import { occasionAt, occasionTitle, SLOT_ORDER } from './plannerWeek';
import type { OccasionView, PlannerWeek } from './types';

function dayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'short' });
}

export function MoveCopyDialog({
  mode,
  occasion,
  week,
  busy,
  onConfirm,
  onClose,
}: {
  mode: 'move' | 'copy';
  occasion: OccasionView;
  week: PlannerWeek;
  busy: boolean;
  onConfirm: (date: string, slot: MealSlot) => void;
  onClose: () => void;
}) {
  const [date, setDate] = useState(addDays(occasion.planned_on, 1));
  const [slot, setSlot] = useState<MealSlot>(occasion.slot);
  const days = Array.from({ length: 14 }, (_, index) => addDays(week.week_start, index));
  const sameWeek = date >= week.week_start && date <= addDays(week.week_start, 6);
  const taken = sameWeek ? occasionAt(week, date, slot) !== null : false;

  return (
    <FormDialog open onClose={busy ? undefined : onClose} fullWidth maxWidth="xs">
      <DialogTitle>{mode === 'move' ? 'Move meal' : 'Copy to day'}</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ pt: 0.5 }}>
          <Typography variant="body2" color="text.secondary">
            {occasionTitle(occasion)}
          </Typography>
          <TextField select label="Day" value={date} onChange={(event) => setDate(event.target.value)} fullWidth>
            {days.map((candidate) => (
              <MenuItem key={candidate} value={candidate}>
                {dayLabel(candidate)}
              </MenuItem>
            ))}
          </TextField>
          <TextField select label="Meal" value={slot} onChange={(event) => setSlot(event.target.value as MealSlot)} fullWidth>
            {SLOT_ORDER.map((candidate) => (
              <MenuItem key={candidate} value={candidate}>
                {labelForSlot(candidate)}
              </MenuItem>
            ))}
          </TextField>
          {taken ? (
            <Typography variant="body2" color="text.secondary">
              Something is already planned there.
            </Typography>
          ) : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={busy}>
          Cancel
        </Button>
        <Button variant="contained" onClick={() => onConfirm(date, slot)} disabled={busy || taken}>
          {mode === 'move' ? 'Move' : 'Copy'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
