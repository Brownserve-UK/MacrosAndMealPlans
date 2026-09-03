import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Grid from '@mui/material/Grid';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import { useState } from 'react';
import { ApiError, type NutritionGoals, type NutritionTarget } from '../../api/client';
import {
  useCreateNutritionTarget,
  useDeleteNutritionTarget,
  useNutritionTargets,
  useUpdateNutritionTarget,
} from '../../api/queries';
import type { components } from '../../api/schema';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { ErrorState, Loading } from '../../components/States';

type NutrientKey = keyof NutritionGoals;

type Field = { key: NutrientKey; label: string; unit: string };

const GROUPS: { title: string; fields: Field[] }[] = [
  { title: 'Energy', fields: [{ key: 'energy_kcal', label: 'Calories', unit: 'kcal' }] },
  {
    title: 'Macros',
    fields: [
      { key: 'protein_g', label: 'Protein', unit: 'g' },
      { key: 'carbohydrate_g', label: 'Carbs', unit: 'g' },
      { key: 'fat_g', label: 'Fat', unit: 'g' },
    ],
  },
  {
    title: 'Dietary',
    fields: [
      { key: 'sugar_g', label: 'Sugars', unit: 'g' },
      { key: 'saturated_fat_g', label: 'Saturates', unit: 'g' },
      { key: 'fibre_g', label: 'Fibre', unit: 'g' },
      { key: 'salt_g', label: 'Salt', unit: 'g' },
      { key: 'cholesterol_mg', label: 'Cholesterol', unit: 'mg' },
    ],
  },
];

const FIELDS: Field[] = GROUPS.flatMap((group) => group.fields);

function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

function formatDate(iso: string) {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

function goalsToDraft(goals: NutritionGoals): Record<string, string> {
  const draft: Record<string, string> = {};
  for (const { key } of FIELDS) {
    const value = goals[key];
    draft[key] = value == null ? '' : String(value);
  }
  return draft;
}

type CreateBody = components['schemas']['CreateNutritionTargetRequest'];
type UpdateBody = components['schemas']['UpdateNutritionTargetRequest'];

function currentTarget(targets: NutritionTarget[]): NutritionTarget | undefined {
  return targets.at(-1);
}

export function NutritionTargetsPanel({ memberId }: { memberId: string }) {
  const query = useNutritionTargets(memberId);

  if (query.isLoading) return <Loading label="Loading targets" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const targets = query.data ?? [];
  const current = currentTarget(targets);

  return (
    <TargetsView memberId={memberId} targets={targets} current={current} />
  );
}

type DialogState =
  | { mode: 'create'; from?: NutritionGoals }
  | { mode: 'edit'; target: NutritionTarget }
  | null;

function TargetsView({
  memberId,
  targets,
  current,
}: {
  memberId: string;
  targets: NutritionTarget[];
  current: NutritionTarget | undefined;
}) {
  const [dialog, setDialog] = useState<DialogState>(null);
  const [removing, setRemoving] = useState<NutritionTarget | null>(null);

  return (
    <Paper sx={{ p: 3 }}>
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', mb: 2, gap: 1 }}
      >
        <Typography variant="h3">Nutrition targets</Typography>
        <Button variant="contained" onClick={() => setDialog({ mode: 'create', from: current })}>
          Change targets
        </Button>
      </Stack>

      {current ? (
        <Stack spacing={0.75} sx={{ mb: targets.length > 1 ? 2.5 : 0 }}>
          <Typography variant="caption" color="text.secondary">
            In force since {formatDate(current.effective_from)}
          </Typography>
          <Stack direction="row" spacing={2} sx={{ flexWrap: 'wrap', gap: 1 }}>
            {FIELDS.filter((field) => current[field.key] != null).map((field) => (
              <Typography key={field.key} className="numeral" variant="body2">
                <Typography component="span" variant="caption" color="text.secondary">
                  {field.label}{' '}
                </Typography>
                {current[field.key]}
                <Typography component="span" variant="caption" color="text.secondary">
                  {' '}
                  {field.unit}
                </Typography>
              </Typography>
            ))}
          </Stack>
        </Stack>
      ) : (
        <Typography variant="body2" color="text.secondary">
          You haven't set any targets yet. Your meal plan will show progress against them once you do.
        </Typography>
      )}

      {targets.length > 0 ? (
        <>
          <Divider sx={{ my: 2 }} />
          <Typography variant="overline" color="text.secondary">
            History
          </Typography>
          <Stack divider={<Divider />} sx={{ mt: 1 }}>
            {[...targets].reverse().map((target) => (
              <Stack
                key={target.id}
                direction="row"
                sx={{ alignItems: 'center', justifyContent: 'space-between', py: 1, gap: 1 }}
              >
                <Stack sx={{ minWidth: 0 }}>
                  <Typography variant="body2">From {formatDate(target.effective_from)}</Typography>
                  <Typography className="numeral" variant="caption" color="text.secondary">
                    {target.energy_kcal != null
                      ? `${target.energy_kcal} kcal`
                      : 'No calorie target'}
                  </Typography>
                </Stack>
                <Stack direction="row" spacing={0.5}>
                  <IconButton
                    size="small"
                    aria-label={`Edit target from ${formatDate(target.effective_from)}`}
                    onClick={() => setDialog({ mode: 'edit', target })}
                  >
                    <EditIcon fontSize="small" />
                  </IconButton>
                  <IconButton
                    size="small"
                    aria-label={`Delete target from ${formatDate(target.effective_from)}`}
                    onClick={() => setRemoving(target)}
                  >
                    <DeleteIcon fontSize="small" />
                  </IconButton>
                </Stack>
              </Stack>
            ))}
          </Stack>
        </>
      ) : null}

      {dialog ? (
        <TargetDialog
          memberId={memberId}
          state={dialog}
          onClose={() => setDialog(null)}
        />
      ) : null}

      {removing ? (
        <DeleteTargetDialog
          memberId={memberId}
          target={removing}
          hasEarlier={targets.some((t) => t.effective_from < removing.effective_from)}
          onClose={() => setRemoving(null)}
        />
      ) : null}
    </Paper>
  );
}

function draftErrors(draft: Record<string, string>): { errors: Record<string, string>; any: boolean } {
  const errors: Record<string, string> = {};
  let any = false;
  for (const { key } of FIELDS) {
    const raw = (draft[key] ?? '').trim();
    if (raw === '') continue;
    any = true;
    const parsed = Number(raw);
    if (Number.isNaN(parsed)) errors[key] = 'Must be a number';
    else if (parsed < 0) errors[key] = 'Must not be negative';
  }
  return { errors, any };
}

function TargetDialog({
  memberId,
  state,
  onClose,
}: {
  memberId: string;
  state: Exclude<DialogState, null>;
  onClose: () => void;
}) {
  const editing = state.mode === 'edit';
  const create = useCreateNutritionTarget();
  const update = useUpdateNutritionTarget();

  const [effectiveFrom, setEffectiveFrom] = useState(() =>
    editing ? state.target.effective_from : todayIso(),
  );
  const [draft, setDraft] = useState<Record<string, string>>(() =>
    goalsToDraft(editing ? state.target : state.from ?? {}),
  );
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  const pending = create.isPending || update.isPending;

  function set(key: string, value: string) {
    setDraft((prev) => ({ ...prev, [key]: value }));
  }

  async function onSubmit() {
    setFormError(null);
    const { errors: fieldErrors, any } = draftErrors(draft);
    if (!effectiveFrom) fieldErrors.effective_from = 'Pick a date';
    if (!any) {
      setErrors(fieldErrors);
      setFormError('Set at least one target.');
      return;
    }
    setErrors(fieldErrors);
    if (Object.keys(fieldErrors).length > 0) return;

    try {
      if (editing) {
        const body: UpdateBody = { effective_from: effectiveFrom };
        for (const { key } of FIELDS) {
          const raw = (draft[key] ?? '').trim();
          body[key] = raw === '' ? null : Number(raw);
        }
        await update.mutateAsync({ id: state.target.id, revision: state.target.revision, body });
      } else {
        const body: CreateBody = { effective_from: effectiveFrom };
        for (const { key } of FIELDS) {
          const raw = (draft[key] ?? '').trim();
          if (raw !== '') body[key] = Number(raw);
        }
        await create.mutateAsync({ memberId, body });
      }
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else if (caught.status === 409) setFormError('A target already takes effect on that date.');
        else setFormError(Object.values(caught.fieldErrors)[0] ?? caught.message);
      } else {
        setFormError('Something went wrong.');
      }
    }
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>{editing ? 'Edit target' : 'Change targets'}</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {editing ? (
            <Alert severity="warning">
              This is a past target. Editing it changes the meal-plan comparisons for the days it
              covered.
            </Alert>
          ) : (
            <Typography variant="body2" color="text.secondary">
              This starts a new target from the date you pick. Earlier days keep the target that was
              in force then.
            </Typography>
          )}

          <TextField
            label="In force from"
            type="date"
            value={effectiveFrom}
            onChange={(e) => setEffectiveFrom(e.target.value)}
            error={Boolean(errors.effective_from)}
            helperText={errors.effective_from}
            slotProps={{ inputLabel: { shrink: true } }}
            fullWidth
          />

          {GROUPS.map((group) => (
            <Stack key={group.title} spacing={1}>
              <Typography variant="overline" color="text.secondary">
                {group.title}
              </Typography>
              <Grid container spacing={2}>
                {group.fields.map((field) => (
                  <Grid key={field.key} size={{ xs: 6, sm: 4 }}>
                    <TextField
                      label={field.label}
                      value={draft[field.key] ?? ''}
                      onChange={(e) => set(field.key, e.target.value)}
                      error={Boolean(errors[field.key])}
                      helperText={errors[field.key]}
                      placeholder="—"
                      inputMode="decimal"
                      slotProps={{
                        input: {
                          endAdornment: (
                            <Typography variant="caption" color="text.secondary">
                              {field.unit}
                            </Typography>
                          ),
                        },
                      }}
                      fullWidth
                    />
                  </Grid>
                ))}
              </Grid>
            </Stack>
          ))}

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

function DeleteTargetDialog({
  memberId,
  target,
  hasEarlier,
  onClose,
}: {
  memberId: string;
  target: NutritionTarget;
  hasEarlier: boolean;
  onClose: () => void;
}) {
  const remove = useDeleteNutritionTarget();
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function onConfirm() {
    setError(null);
    try {
      await remove.mutateAsync({ id: target.id, revision: target.revision, memberId });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
      else setError('Could not delete this target.');
    }
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>Delete this target?</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2}>
          <Typography variant="body2">
            The target from {formatDate(target.effective_from)} will be removed. This changes the
            meal-plan comparisons for the days it covered.
          </Typography>
          {hasEarlier ? (
            <Typography variant="body2" color="text.secondary">
              An earlier target will apply to those days again.
            </Typography>
          ) : null}
          {error ? <Alert severity="error">{error}</Alert> : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button color="error" variant="contained" onClick={onConfirm} disabled={remove.isPending}>
          {remove.isPending ? 'Deleting…' : 'Delete'}
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
