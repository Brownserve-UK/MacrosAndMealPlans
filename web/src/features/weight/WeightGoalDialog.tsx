import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import {
  ApiError,
  type WeightDisplay,
  type WeightGoal,
  type WeightObjective,
} from '../../api/client';
import { useClearWeightGoal, useSetWeightGoal, useUpdateWeightGoal } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import {
  EMPTY_INPUT,
  parseRateInput,
  parseWeightInput,
  rateToInput,
  toInput,
  unitLabel,
  type WeightInput,
} from './weightFormat';

const OBJECTIVES: { value: WeightObjective; label: string }[] = [
  { value: 'lose', label: 'Lose weight' },
  { value: 'maintain', label: 'Stay where I am' },
  { value: 'gain', label: 'Gain weight' },
];

function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

function WeightFields({
  label,
  display,
  value,
  error,
  onChange,
}: {
  label: string;
  display: WeightDisplay;
  value: WeightInput;
  error: string | null;
  onChange: (next: WeightInput) => void;
}) {
  const stones = display === 'stones_pounds';
  return (
    <Stack direction="row" spacing={1.5}>
      <TextField
        label={stones ? `${label} (st)` : label}
        value={value.primary}
        onChange={(e) => onChange({ ...value, primary: e.target.value })}
        error={Boolean(error)}
        helperText={error}
        inputMode="decimal"
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
          value={value.secondary}
          onChange={(e) => onChange({ ...value, secondary: e.target.value })}
          error={Boolean(error)}
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
  );
}

export function WeightGoalDialog({
  memberId,
  display,
  goal,
  currentWeightKg,
  onClose,
}: {
  memberId: string;
  display: WeightDisplay;
  goal: WeightGoal | null;
  currentWeightKg: number | null;
  onClose: () => void;
}) {
  const set = useSetWeightGoal();
  const update = useUpdateWeightGoal();
  const clear = useClearWeightGoal();

  const [objective, setObjective] = useState<WeightObjective>(goal?.objective ?? 'lose');
  const [starting, setStarting] = useState<WeightInput>(() =>
    toInput(goal?.starting_weight_kg ?? currentWeightKg, display),
  );
  const [target, setTarget] = useState<WeightInput>(() =>
    goal?.target_weight_kg != null ? toInput(goal.target_weight_kg, display) : EMPTY_INPUT,
  );
  const [rate, setRate] = useState(() => rateToInput(goal?.planned_rate_kg_per_week, display));
  const [startedOn, setStartedOn] = useState(() => goal?.started_on ?? todayIso());
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  const maintaining = objective === 'maintain';
  const pending = set.isPending || update.isPending || clear.isPending;

  async function onClear() {
    if (!goal) return;
    setFormError(null);
    try {
      await clear.mutateAsync({ id: goal.id, revision: goal.revision, memberId });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
      else setFormError('Could not clear your goal.');
    }
  }

  async function onSubmit() {
    setFormError(null);
    const next: Record<string, string> = {};

    const startingWeight = parseWeightInput(starting, display);
    if (!startingWeight) next.starting = 'Enter a weight';

    const targetWeight = maintaining ? null : parseWeightInput(target, display);
    if (!maintaining && !targetWeight) next.target = 'Enter a weight';

    const plannedRate = maintaining ? null : parseRateInput(rate, display);
    if (!maintaining && !plannedRate) next.rate = 'Enter a rate';

    if (!startedOn) next.startedOn = 'Pick a date';

    setErrors(next);
    if (Object.keys(next).length > 0 || !startingWeight) return;

    try {
      if (goal) {
        await update.mutateAsync({
          id: goal.id,
          revision: goal.revision,
          body: {
            objective,
            starting_weight: startingWeight,
            target_weight: targetWeight,
            planned_rate: plannedRate,
            started_on: startedOn,
          },
        });
      } else {
        await set.mutateAsync({
          memberId,
          body: {
            objective,
            starting_weight: startingWeight,
            target_weight: targetWeight ?? undefined,
            planned_rate: plannedRate ?? undefined,
            started_on: startedOn,
          },
        });
      }
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else if (caught.status === 409) setFormError('You already have a goal.');
        else setFormError(Object.values(caught.fieldErrors)[0] ?? caught.message);
      } else {
        setFormError('Something went wrong.');
      }
    }
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>{goal ? 'Edit goal' : 'Set a goal'}</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          <TextField
            select
            label="I want to"
            value={objective}
            onChange={(e) => setObjective(e.target.value as WeightObjective)}
            fullWidth
          >
            {OBJECTIVES.map((option) => (
              <MenuItem key={option.value} value={option.value}>
                {option.label}
              </MenuItem>
            ))}
          </TextField>

          <WeightFields
            label="Starting weight"
            display={display}
            value={starting}
            error={errors.starting ?? null}
            onChange={setStarting}
          />

          {maintaining ? null : (
            <>
              <WeightFields
                label="Target weight"
                display={display}
                value={target}
                error={errors.target ?? null}
                onChange={setTarget}
              />

              <TextField
                label="Each week"
                value={rate}
                onChange={(e) => setRate(e.target.value)}
                error={Boolean(errors.rate)}
                helperText={errors.rate}
                inputMode="decimal"
                slotProps={{
                  input: {
                    endAdornment: (
                      <Typography variant="caption" color="text.secondary">
                        {unitLabel(display)} a week
                      </Typography>
                    ),
                  },
                }}
                fullWidth
              />
            </>
          )}

          <TextField
            label="Started"
            type="date"
            value={startedOn}
            onChange={(e) => setStartedOn(e.target.value)}
            error={Boolean(errors.startedOn)}
            helperText={errors.startedOn}
            slotProps={{ inputLabel: { shrink: true } }}
            fullWidth
          />

          {formError ? <Alert severity="error">{formError}</Alert> : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        {goal ? (
          <Button color="error" onClick={onClear} disabled={pending} sx={{ mr: 'auto' }}>
            Clear goal
          </Button>
        ) : null}
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
