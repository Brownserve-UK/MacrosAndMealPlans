import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import type {
  BodyProfile,
  CalorieCalculation,
  HabitualActivity,
  Pace,
  WeightObjective,
  WeightSummary,
} from '../../api/client';
import { ApiError } from '../../api/client';
import { usePreviewCalorieTarget, useSetGuidedCalorieTarget } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { paceAdjustment, paceOptionsFor } from './pace';

type Step = 'about' | 'activity' | 'goal' | 'recommendation';

type Draft = {
  dateOfBirth: string;
  sex: 'male' | 'female' | '';
  heightCm: string;
  currentWeight: string;
  habitualActivity: HabitualActivity | '';
  objective: WeightObjective;
  targetWeight: string;
  pace: Pace;
};

const ACTIVITIES: { value: HabitualActivity; label: string; description: string }[] = [
  { value: 'mostly_sedentary', label: 'Mostly sedentary', description: 'Most days are spent sitting.' },
  {
    value: 'lightly_active',
    label: 'Lightly active',
    description: 'Some walking and moving around most days.',
  },
  { value: 'active', label: 'Active', description: 'On your feet for much of the day.' },
  {
    value: 'very_active',
    label: 'Very active',
    description: 'Physical work or lots of everyday movement.',
  },
];

function initialDraft(profile: BodyProfile | null | undefined, summary: WeightSummary): Draft {
  return {
    dateOfBirth: profile?.date_of_birth ?? '',
    sex: profile?.sex ?? '',
    heightCm: profile?.height_cm == null ? '' : String(profile.height_cm),
    currentWeight: summary.latest?.weight_kg == null ? '' : String(summary.latest.weight_kg),
    habitualActivity: profile?.habitual_activity ?? '',
    objective: summary.goal?.objective ?? 'lose',
    targetWeight: summary.goal?.target_weight_kg == null ? '' : String(summary.goal.target_weight_kg),
    pace: 'standard',
  };
}

function formatDate(iso: string) {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

function estimatedGoalDate(draft: Draft, calculation: CalorieCalculation): string | null {
  if (draft.objective === 'maintain' || !calculation.applied_rate_kg_per_week) return null;
  const distance = Math.abs(Number(draft.targetWeight) - Number(draft.currentWeight));
  if (!Number.isFinite(distance)) return null;
  const days = Math.ceil((distance / calculation.applied_rate_kg_per_week) * 7);
  const date = new Date(`${calculation.calculated_on}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

function answers(draft: Draft) {
  return {
    date_of_birth: draft.dateOfBirth,
    sex: draft.sex as 'male' | 'female',
    height_cm: Number(draft.heightCm),
    current_weight: { amount: Number(draft.currentWeight), unit: 'kg' as const },
    habitual_activity: draft.habitualActivity as HabitualActivity,
    objective: draft.objective,
    target_weight:
      draft.objective === 'maintain' ? null : { amount: Number(draft.targetWeight), unit: 'kg' as const },
    pace: draft.objective === 'maintain' ? null : draft.pace,
  };
}

export function GuidedSetupDialog({
  memberId,
  profile,
  summary,
  onClose,
  onManual,
}: {
  memberId: string;
  profile?: BodyProfile | null;
  summary: WeightSummary;
  onClose: () => void;
  onManual: () => void;
}) {
  const preview = usePreviewCalorieTarget();
  const save = useSetGuidedCalorieTarget();
  const [step, setStep] = useState<Step>('about');
  const [draft, setDraft] = useState(() => initialDraft(profile, summary));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [calculation, setCalculation] = useState<CalorieCalculation | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  function set<K extends keyof Draft>(key: K, value: Draft[K]) {
    setDraft((previous) => ({ ...previous, [key]: value }));
  }

  function validateAbout() {
    const next: Record<string, string> = {};
    if (!draft.dateOfBirth) next.dateOfBirth = 'Enter your date of birth';
    if (!draft.sex) next.sex = 'Choose an option';
    if (!Number.isFinite(Number(draft.heightCm)) || Number(draft.heightCm) <= 0) {
      next.heightCm = 'Enter your height';
    }
    if (!Number.isFinite(Number(draft.currentWeight)) || Number(draft.currentWeight) <= 0) {
      next.currentWeight = 'Enter your weight';
    }
    return next;
  }

  function validateGoal() {
    const next: Record<string, string> = {};
    if (draft.objective === 'maintain') return next;
    const current = Number(draft.currentWeight);
    const target = Number(draft.targetWeight);
    if (!Number.isFinite(target) || target <= 0) {
      next.targetWeight = 'Enter a goal weight';
    } else if (draft.objective === 'lose' && target >= current) {
      next.targetWeight = 'Choose a lower weight than your current weight';
    } else if (draft.objective === 'gain' && target <= current) {
      next.targetWeight = 'Choose a higher weight than your current weight';
    }
    return next;
  }

  async function next(event: FormEvent) {
    event.preventDefault();
    let nextErrors: Record<string, string> = {};
    if (step === 'about') nextErrors = validateAbout();
    if (step === 'activity' && !draft.habitualActivity) nextErrors.habitualActivity = 'Choose an option';
    if (step === 'goal') nextErrors = validateGoal();
    setErrors(nextErrors);
    setFormError(null);
    if (Object.keys(nextErrors).length > 0) return;

    if (step === 'about') setStep('activity');
    else if (step === 'activity') setStep('goal');
    else if (step === 'goal') {
      try {
        const result = await preview.mutateAsync({ id: memberId, body: answers(draft) });
        setCalculation(result);
        setStep('recommendation');
      } catch (caught) {
        setFormError(
          caught instanceof ApiError
            ? (Object.values(caught.fieldErrors)[0] ?? caught.message)
            : 'Something went wrong.',
        );
      }
    }
  }

  async function saveTarget() {
    try {
      await save.mutateAsync({ id: memberId, body: answers(draft) });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) {
        setConflict(caught);
      } else {
        setFormError(
          caught instanceof ApiError
            ? (Object.values(caught.fieldErrors)[0] ?? caught.message)
            : 'Something went wrong.',
        );
      }
    }
  }

  const adjustment = calculation ? Math.abs(calculation.adjustment_kcal).toLocaleString('en-GB') : '';
  const goalDate = calculation ? estimatedGoalDate(draft, calculation) : null;

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="sm">
      <form onSubmit={next}>
        <DialogTitle>
          {step === 'about'
            ? 'About you'
            : step === 'activity'
              ? 'Your usual day'
              : step === 'goal'
                ? 'Your goal'
                : 'Your target'}
        </DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2.5}>
            {step === 'about' ? (
              <>
                <TextField
                  label="Date of birth"
                  type="date"
                  value={draft.dateOfBirth}
                  onChange={(event) => set('dateOfBirth', event.target.value)}
                  error={Boolean(errors.dateOfBirth)}
                  helperText={errors.dateOfBirth}
                  slotProps={{ inputLabel: { shrink: true } }}
                  fullWidth
                />
                <TextField
                  select
                  label="Sex"
                  value={draft.sex}
                  onChange={(event) => set('sex', event.target.value as Draft['sex'])}
                  error={Boolean(errors.sex)}
                  helperText={errors.sex ?? 'Used to estimate your energy needs'}
                  fullWidth
                >
                  <MenuItem value="male">Male</MenuItem>
                  <MenuItem value="female">Female</MenuItem>
                </TextField>
                <TextField
                  label="Height"
                  value={draft.heightCm}
                  onChange={(event) => set('heightCm', event.target.value)}
                  error={Boolean(errors.heightCm)}
                  helperText={errors.heightCm}
                  inputMode="decimal"
                  slotProps={{
                    input: {
                      endAdornment: (
                        <Typography variant="caption" color="text.secondary">
                          cm
                        </Typography>
                      ),
                    },
                  }}
                  fullWidth
                />
                <TextField
                  label="Current weight"
                  value={draft.currentWeight}
                  onChange={(event) => set('currentWeight', event.target.value)}
                  error={Boolean(errors.currentWeight)}
                  helperText={errors.currentWeight}
                  inputMode="decimal"
                  slotProps={{
                    input: {
                      endAdornment: (
                        <Typography variant="caption" color="text.secondary">
                          kg
                        </Typography>
                      ),
                    },
                  }}
                  fullWidth
                />
              </>
            ) : null}

            {step === 'activity' ? (
              <>
                <ToggleButtonGroup
                  exclusive
                  value={draft.habitualActivity}
                  onChange={(_event, value: HabitualActivity | null) => value && set('habitualActivity', value)}
                  orientation="vertical"
                  aria-label="Your usual activity"
                >
                  {ACTIVITIES.map((activity) => (
                    <ToggleButton
                      key={activity.value}
                      value={activity.value}
                      sx={{ justifyContent: 'flex-start', textAlign: 'left', py: 1.5 }}
                    >
                      <Stack>
                        <Typography variant="body2">{activity.label}</Typography>
                        <Typography variant="caption" color="text.secondary">
                          {activity.description}
                        </Typography>
                      </Stack>
                    </ToggleButton>
                  ))}
                </ToggleButtonGroup>
                {errors.habitualActivity ? (
                  <Typography variant="caption" color="error">
                    {errors.habitualActivity}
                  </Typography>
                ) : null}
                <Typography variant="body2" color="text.secondary">
                  Don&apos;t count workouts here.
                </Typography>
              </>
            ) : null}

            {step === 'goal' ? (
              <>
                <ToggleButtonGroup
                  exclusive
                  value={draft.objective}
                  onChange={(_event, value: WeightObjective | null) => value && set('objective', value)}
                  aria-label="Your goal"
                >
                  <ToggleButton value="lose">Lose</ToggleButton>
                  <ToggleButton value="maintain">Maintain</ToggleButton>
                  <ToggleButton value="gain">Gain</ToggleButton>
                </ToggleButtonGroup>
                {draft.objective !== 'maintain' ? (
                  <>
                    <TextField
                      label="Goal weight"
                      value={draft.targetWeight}
                      onChange={(event) => set('targetWeight', event.target.value)}
                      error={Boolean(errors.targetWeight)}
                      helperText={errors.targetWeight}
                      inputMode="decimal"
                      slotProps={{
                        input: {
                          endAdornment: (
                            <Typography variant="caption" color="text.secondary">
                              kg
                            </Typography>
                          ),
                        },
                      }}
                      fullWidth
                    />
                    <ToggleButtonGroup
                      exclusive
                      value={draft.pace}
                      onChange={(_event, value: Pace | null) => value && set('pace', value)}
                      orientation="vertical"
                      aria-label="Pace"
                    >
                      {paceOptionsFor(draft.objective).map((pace) => (
                        <ToggleButton key={pace.value} value={pace.value} sx={{ justifyContent: 'space-between' }}>
                          <Typography variant="body2">{pace.label}</Typography>
                          <Typography className="numeral" variant="caption" color="text.secondary">
                            {paceAdjustment(pace.value, draft.objective)}
                          </Typography>
                        </ToggleButton>
                      ))}
                    </ToggleButtonGroup>
                  </>
                ) : null}
              </>
            ) : null}

            {step === 'recommendation' && calculation ? (
              <>
                <Typography variant="caption" color="text.secondary">
                  Your daily target
                </Typography>
                <Typography className="numeral" variant="h2">
                  {calculation.recommended_kcal.toLocaleString('en-GB')} kcal
                </Typography>
                <Typography className="numeral" variant="body2">
                  Maintenance {calculation.maintenance_kcal.toLocaleString('en-GB')} kcal ·{' '}
                  {calculation.adjustment_kcal === 0
                    ? 'no daily adjustment'
                    : `${adjustment} kcal ${calculation.adjustment_kcal < 0 ? 'less' : 'more'} a day`}
                </Typography>
                {goalDate ? (
                  <Typography className="numeral" variant="body2">
                    On track for {formatDate(goalDate)}
                  </Typography>
                ) : null}
                {calculation.eased ? (
                  <Alert severity="warning">
                    We eased this pace to keep your target at the safety floor of{' '}
                    <span className="numeral">{calculation.floor_kcal.toLocaleString('en-GB')} kcal</span>.
                  </Alert>
                ) : null}
                <Typography variant="body2" color="text.secondary">
                  General guidance, not advice from a nutrition professional.
                </Typography>
              </>
            ) : null}

            {formError ? <Alert severity="error">{formError}</Alert> : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          {step === 'recommendation' ? (
            <>
              <Button onClick={() => setStep('goal')}>Change my answers</Button>
              <Button onClick={onManual}>Set a different number</Button>
              <Button variant="contained" onClick={() => void saveTarget()} disabled={save.isPending}>
                {save.isPending ? 'Saving…' : 'Use this target'}
              </Button>
            </>
          ) : (
            <>
              <Button onClick={onClose}>Cancel</Button>
              <Button type="submit" variant="contained" disabled={preview.isPending}>
                {step === 'goal' ? (preview.isPending ? 'Working it out…' : 'See my target') : 'Continue'}
              </Button>
            </>
          )}
        </DialogActions>
      </form>
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
