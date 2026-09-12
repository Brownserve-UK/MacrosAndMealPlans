import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import { useState } from 'react';
import type { NutritionPlan, WeightDisplay, WeightRecord, WeightSummary } from '../../api/client';
import { useAuth } from '../../auth/AuthProvider';
import {
  useBodyProfile,
  useDeleteWeighIn,
  useMember,
  useNutritionPlan,
  useUpdateMember,
  useWeightRecords,
  useWeightSummary,
} from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { GuidedSetupDialog } from './GuidedSetupDialog';
import { ManualTargetDialog } from './ManualTargetDialog';
import { WeighInDialog, type WeighInDialogState } from './WeighInDialog';
import { WeightChart } from './WeightChart';
import { formatWeight } from './weightFormat';

const DISPLAYS: { value: WeightDisplay; label: string }[] = [
  { value: 'kilograms', label: 'kg' },
  { value: 'stones_pounds', label: 'st + lb' },
  { value: 'pounds', label: 'lb' },
];

type DialogState = 'guided' | 'manual' | null;

function formatDate(iso: string) {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

function formatCalories(value: number) {
  return `${value.toLocaleString('en-GB')} kcal`;
}

export function GoalsPage() {
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const member = useMember(memberId);
  const plan = useNutritionPlan(memberId);
  const summary = useWeightSummary(memberId);
  const records = useWeightRecords(memberId);
  const profile = useBodyProfile(memberId);

  if (!principal) return null;
  if (!memberId) {
    return (
      <>
        <PageHeader title="Goals" />
        <Typography variant="body2" color="text.secondary">
          Your account is not linked to a household member yet.
        </Typography>
      </>
    );
  }
  if (member.isLoading || plan.isLoading || summary.isLoading) return <Loading label="Loading goals" />;
  if (member.isError) return <ErrorState error={member.error} onRetry={() => member.refetch()} />;
  if (plan.isError) return <ErrorState error={plan.error} onRetry={() => plan.refetch()} />;
  if (summary.isError) return <ErrorState error={summary.error} onRetry={() => summary.refetch()} />;

  return (
    <GoalsView
      memberId={memberId}
      display={member.data?.weight_display ?? 'kilograms'}
      memberRevision={member.data?.revision ?? 0}
      plan={plan.data as NutritionPlan}
      summary={summary.data as WeightSummary}
      records={records.data ?? []}
      profile={profile.data}
    />
  );
}

function GoalsView({
  memberId,
  display,
  memberRevision,
  plan,
  summary,
  records,
  profile,
}: {
  memberId: string;
  display: WeightDisplay;
  memberRevision: number;
  plan: NutritionPlan;
  summary: WeightSummary;
  records: WeightRecord[];
  profile: ReturnType<typeof useBodyProfile>['data'];
}) {
  const updateMember = useUpdateMember();
  const [dialog, setDialog] = useState<DialogState>(null);
  const [weighIn, setWeighIn] = useState<WeighInDialogState | null>(null);
  const [removing, setRemoving] = useState<WeightRecord | null>(null);
  const target = plan.target?.energy_kcal ?? null;
  const calculation = plan.calculation ?? null;

  return (
    <>
      <PageHeader
        title="Goals"
        actions={
          <Stack direction="row" spacing={1}>
            <ToggleButtonGroup
              size="small"
              exclusive
              value={display}
              onChange={(_event, next: WeightDisplay | null) => {
                if (!next || next === display) return;
                updateMember.mutate({
                  id: memberId,
                  revision: memberRevision,
                  body: { weight_display: next },
                });
              }}
              aria-label="Show weights in"
            >
              {DISPLAYS.map((option) => (
                <ToggleButton key={option.value} value={option.value}>
                  {option.label}
                </ToggleButton>
              ))}
            </ToggleButtonGroup>
            <Button variant="contained" onClick={() => setWeighIn({ mode: 'create' })}>
              Add weigh-in
            </Button>
          </Stack>
        }
      />

      <Stack spacing={3}>
        {target == null ? (
          <Paper sx={{ p: 3 }}>
            <Stack spacing={2}>
              <Typography variant="body2" color="text.secondary">
                We can work out a daily calorie target by asking a few questions.
              </Typography>
              <Stack direction="row" spacing={1}>
                <Button variant="contained" onClick={() => setDialog('guided')}>
                  Start
                </Button>
                <Button onClick={() => setDialog('manual')}>Set my own target</Button>
              </Stack>
            </Stack>
          </Paper>
        ) : (
          <>
            <Paper sx={{ p: 3 }}>
              <Stack spacing={1.5}>
                <Typography variant="caption" color="text.secondary">
                  Your daily target
                </Typography>
                <Typography className="numeral" variant="h2">
                  {formatCalories(target)}
                </Typography>
                <Stack direction="row" spacing={1}>
                  <Button variant="contained" onClick={() => setDialog('guided')}>
                    Review my plan
                  </Button>
                  <Button onClick={() => setDialog('manual')}>Set a different number</Button>
                </Stack>
              </Stack>
            </Paper>

            {calculation ? (
              <Paper sx={{ p: 3 }}>
                <Stack spacing={1}>
                  <Typography variant="h3">How this was worked out</Typography>
                  <Typography className="numeral" variant="body2">
                    Worked out {formatDate(calculation.calculated_on)} from{' '}
                    {formatWeight(calculation.weight_kg, display)}
                  </Typography>
                  <Typography className="numeral" variant="body2" color="text.secondary">
                    Maintenance {formatCalories(calculation.maintenance_kcal)} ·{' '}
                    {Math.abs(calculation.adjustment_kcal).toLocaleString('en-GB')} kcal{' '}
                    {calculation.adjustment_kcal < 0
                      ? 'less'
                      : calculation.adjustment_kcal > 0
                        ? 'more'
                        : 'adjustment'}{' '}
                    a day
                  </Typography>
                  {calculation.eased ? (
                    <Alert severity="warning">
                      Your pace was eased to the safety floor of{' '}
                      <span className="numeral">{formatCalories(calculation.floor_kcal)}</span>.
                    </Alert>
                  ) : null}
                </Stack>
              </Paper>
            ) : null}
          </>
        )}

        {summary.projection?.status === 'projected' ? (
          <Paper sx={{ p: 3 }}>
            <Typography variant="h3">Estimated goal date</Typography>
            <Typography className="numeral" variant="body1" sx={{ mt: 1 }}>
              {formatDate(summary.projection.on)}
            </Typography>
          </Paper>
        ) : null}

        {summary.series.length > 0 ? (
          <Paper sx={{ p: 3 }}>
            <Typography variant="h3" sx={{ mb: 2 }}>
              Trend
            </Typography>
            <WeightChart points={summary.series} goalKg={summary.goal?.target_weight_kg} display={display} />
          </Paper>
        ) : null}

        <Paper sx={{ p: 3 }}>
          <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
            <Typography variant="h3">Weigh-ins</Typography>
            <Button onClick={() => setWeighIn({ mode: 'create' })}>Add weigh-in</Button>
          </Stack>
          {records.length === 0 ? (
            <Typography variant="body2" color="text.secondary">
              Add your first weigh-in to see your trend.
            </Typography>
          ) : (
            <Stack divider={<Divider />}>
              {records.map((record) => (
                <Stack
                  key={record.id}
                  direction="row"
                  sx={{ alignItems: 'center', justifyContent: 'space-between', py: 1, gap: 1 }}
                >
                  <Stack>
                    <Typography className="numeral" variant="body2">
                      {formatWeight(record.weight_kg, display)}
                    </Typography>
                    <Typography className="numeral" variant="caption" color="text.secondary">
                      {formatDate(record.recorded_on)}
                    </Typography>
                  </Stack>
                  <Stack direction="row" spacing={0.5}>
                    <IconButton
                      size="small"
                      aria-label={`Edit weigh-in from ${formatDate(record.recorded_on)}`}
                      onClick={() => setWeighIn({ mode: 'edit', record })}
                    >
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton
                      size="small"
                      aria-label={`Delete weigh-in from ${formatDate(record.recorded_on)}`}
                      onClick={() => setRemoving(record)}
                    >
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  </Stack>
                </Stack>
              ))}
            </Stack>
          )}
        </Paper>
      </Stack>

      {dialog === 'guided' ? (
        <GuidedSetupDialog
          memberId={memberId}
          profile={profile}
          summary={summary}
          onClose={() => setDialog(null)}
          onManual={() => setDialog('manual')}
        />
      ) : null}
      {dialog === 'manual' ? (
        <ManualTargetDialog
          memberId={memberId}
          floorKcal={calculation?.floor_kcal ?? (profile?.sex === 'male' ? 1500 : profile?.sex === 'female' ? 1200 : null)}
          onClose={() => setDialog(null)}
        />
      ) : null}
      {weighIn ? (
        <WeighInDialog memberId={memberId} display={display} state={weighIn} onClose={() => setWeighIn(null)} />
      ) : null}
      {removing ? (
        <DeleteWeighInDialog
          memberId={memberId}
          display={display}
          record={removing}
          onClose={() => setRemoving(null)}
        />
      ) : null}
    </>
  );
}

function DeleteWeighInDialog({
  memberId,
  display,
  record,
  onClose,
}: {
  memberId: string;
  display: WeightDisplay;
  record: WeightRecord;
  onClose: () => void;
}) {
  const remove = useDeleteWeighIn();
  const [error, setError] = useState<string | null>(null);

  async function onConfirm() {
    setError(null);
    try {
      await remove.mutateAsync({ id: record.id, revision: record.revision, memberId });
      onClose();
    } catch {
      setError('Could not delete this weigh-in.');
    }
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>Delete this weigh-in?</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2}>
          <Typography variant="body2">
            <span className="numeral">{formatWeight(record.weight_kg, display)}</span> on{' '}
            <span className="numeral">{formatDate(record.recorded_on)}</span> will be removed from your trend.
          </Typography>
          {error ? <Alert severity="error">{error}</Alert> : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button color="error" onClick={() => void onConfirm()} disabled={remove.isPending}>
          Delete
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
