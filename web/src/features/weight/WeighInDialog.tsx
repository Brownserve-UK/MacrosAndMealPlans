import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type WeightDisplay, type WeightRecord } from '../../api/client';
import { useRecordWeighIn, useUpdateWeighIn } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { EMPTY_INPUT, parseWeightInput, toInput, unitLabel, type WeightInput } from './weightFormat';

function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

export type WeighInDialogState = { mode: 'create' } | { mode: 'edit'; record: WeightRecord };

export function WeighInDialog({
  memberId,
  display,
  state,
  onClose,
}: {
  memberId: string;
  display: WeightDisplay;
  state: WeighInDialogState;
  onClose: () => void;
}) {
  const editing = state.mode === 'edit';
  const create = useRecordWeighIn();
  const update = useUpdateWeighIn();

  const [weight, setWeight] = useState<WeightInput>(() =>
    editing ? toInput(state.record.weight_kg, display) : EMPTY_INPUT,
  );
  const [recordedOn, setRecordedOn] = useState(() =>
    editing ? state.record.recorded_on : todayIso(),
  );
  const [weightError, setWeightError] = useState<string | null>(null);
  const [dateError, setDateError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  const pending = create.isPending || update.isPending;
  const stones = display === 'stones_pounds';

  async function onSubmit() {
    setFormError(null);
    const parsed = parseWeightInput(weight, display);
    setWeightError(parsed ? null : 'Enter a weight');
    setDateError(recordedOn ? null : 'Pick a date');
    if (!parsed || !recordedOn) return;

    try {
      if (editing) {
        await update.mutateAsync({
          id: state.record.id,
          revision: state.record.revision,
          body: { weight: parsed, recorded_on: recordedOn },
        });
      } else {
        await create.mutateAsync({
          memberId,
          body: { weight: parsed, recorded_on: recordedOn },
        });
      }
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else setFormError(Object.values(caught.fieldErrors)[0] ?? caught.message);
      } else {
        setFormError('Something went wrong.');
      }
    }
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>{editing ? 'Edit weigh-in' : 'Add weigh-in'}</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          <Stack direction="row" spacing={1.5}>
            <TextField
              label={stones ? 'Stones' : 'Weight'}
              value={weight.primary}
              onChange={(e) => setWeight((prev) => ({ ...prev, primary: e.target.value }))}
              error={Boolean(weightError)}
              helperText={weightError}
              inputMode="decimal"
              autoFocus
              slotProps={{
                input: {
                  endAdornment: (
                    <Typography variant="caption" color="text.secondary">
                      {stones ? 'st' : unitLabel(display)}
                    </Typography>
                  ),
                },
              }}
              fullWidth
            />
            {stones ? (
              <TextField
                label="Pounds"
                value={weight.secondary}
                onChange={(e) => setWeight((prev) => ({ ...prev, secondary: e.target.value }))}
                error={Boolean(weightError)}
                inputMode="decimal"
                slotProps={{
                  input: {
                    endAdornment: (
                      <Typography variant="caption" color="text.secondary">
                        lb
                      </Typography>
                    ),
                  },
                }}
                fullWidth
              />
            ) : null}
          </Stack>

          <TextField
            label="Date"
            type="date"
            value={recordedOn}
            onChange={(e) => setRecordedOn(e.target.value)}
            error={Boolean(dateError)}
            helperText={dateError}
            slotProps={{ inputLabel: { shrink: true } }}
            fullWidth
          />

          {formError ? <Alert severity="error">{formError}</Alert> : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button variant="contained" onClick={onSubmit} disabled={pending}>
          {pending ? 'Saving…' : 'Save'}
        </Button>
      </DialogActions>

      <ConflictDialog
        error={conflict}
        onReload={() => {
          setConflict(null);
          onClose();
        }}
        onDismiss={() => setConflict(null)}
      />
    </FormDialog>
  );
}
