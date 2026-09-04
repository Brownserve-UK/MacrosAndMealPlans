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
import {
  type GoalProjection,
  type WeightDisplay,
  type WeightGoal,
  type WeightRecord,
  type WeightSummary,
} from '../../api/client';
import {
  useDeleteWeighIn,
  useMember,
  useUpdateMember,
  useWeightRecords,
  useWeightSummary,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { WeighInDialog, type WeighInDialogState } from './WeighInDialog';
import { WeightChart } from './WeightChart';
import { WeightGoalDialog } from './WeightGoalDialog';
import { formatRate, formatWeight, formatWeightChange } from './weightFormat';

const DISPLAYS: { value: WeightDisplay; label: string }[] = [
  { value: 'kilograms', label: 'kg' },
  { value: 'stones_pounds', label: 'st + lb' },
  { value: 'pounds', label: 'lb' },
];

function formatDate(iso: string) {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

export function WeightPage() {
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const member = useMember(memberId);
  const summary = useWeightSummary(memberId);
  const records = useWeightRecords(memberId);

  if (!principal) return null;
  if (!memberId) {
    return (
      <>
        <PageHeader title="Weight" />
        <Paper sx={{ p: 3 }}>
          <Typography variant="body2" color="text.secondary">
            Your account isn't linked to a household member yet, so there is nowhere to keep your
            weight.
          </Typography>
        </Paper>
      </>
    );
  }

  if (summary.isLoading || member.isLoading) return <Loading label="Loading weight" />;
  if (summary.isError) return <ErrorState error={summary.error} onRetry={() => summary.refetch()} />;
  if (member.isError) return <ErrorState error={member.error} onRetry={() => member.refetch()} />;

  return (
    <WeightView
      memberId={memberId}
      display={member.data?.weight_display ?? 'kilograms'}
      memberRevision={member.data?.revision ?? 0}
      summary={summary.data as WeightSummary}
      records={records.data ?? []}
    />
  );
}

function WeightView({
  memberId,
  display,
  memberRevision,
  summary,
  records,
}: {
  memberId: string;
  display: WeightDisplay;
  memberRevision: number;
  summary: WeightSummary;
  records: WeightRecord[];
}) {
  const updateMember = useUpdateMember();
  const [weighIn, setWeighIn] = useState<WeighInDialogState | null>(null);
  const [goalOpen, setGoalOpen] = useState(false);
  const [removing, setRemoving] = useState<WeightRecord | null>(null);

  const latest = summary.latest ?? null;
  const goal = summary.goal ?? null;

  return (
    <>
      <PageHeader
        title="Weight"
        subtitle="Your weigh-ins and how they are tracking against your goal."
        actions={
          <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
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
        <Paper sx={{ p: 3 }}>
          <Headline summary={summary} display={display} onSetGoal={() => setGoalOpen(true)} />
        </Paper>

        {summary.series.length > 0 ? (
          <Paper sx={{ p: 3 }}>
            <Typography variant="h3" sx={{ mb: 2 }}>
              Trend
            </Typography>
            <WeightChart
              points={summary.series}
              goalKg={goal?.target_weight_kg ?? null}
              display={display}
            />
          </Paper>
        ) : null}

        <Paper sx={{ p: 3 }}>
          <Typography variant="h3" sx={{ mb: 2 }}>
            Weigh-ins
          </Typography>
          {records.length === 0 ? (
            <EmptyState
              title="Nothing recorded yet"
              description="Add your first weigh-in and your trend will build from there."
            />
          ) : (
            <Stack divider={<Divider />}>
              {records.map((record) => (
                <Stack
                  key={record.id}
                  direction="row"
                  sx={{ alignItems: 'center', justifyContent: 'space-between', py: 1, gap: 1 }}
                >
                  <Stack sx={{ minWidth: 0 }}>
                    <Typography className="numeral" variant="body2">
                      {formatWeight(record.weight_kg, display)}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
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

      {weighIn ? (
        <WeighInDialog
          memberId={memberId}
          display={display}
          state={weighIn}
          onClose={() => setWeighIn(null)}
        />
      ) : null}

      {goalOpen ? (
        <WeightGoalDialog
          memberId={memberId}
          display={display}
          goal={goal}
          currentWeightKg={latest?.weight_kg ?? null}
          onClose={() => setGoalOpen(false)}
        />
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

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <Stack spacing={0.25} sx={{ minWidth: 0 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography className="numeral" variant="body1">
        {value}
      </Typography>
    </Stack>
  );
}

function projectionText(
  projection: GoalProjection | null | undefined,
  goal: WeightGoal | null,
): string {
  if (!projection || !goal) return '—';
  if (projection.status === 'reached') return 'Reached';
  if (projection.status === 'steady') return 'No end date';
  return formatDate(projection.on);
}

function Headline({
  summary,
  display,
  onSetGoal,
}: {
  summary: WeightSummary;
  display: WeightDisplay;
  onSetGoal: () => void;
}) {
  const latest = summary.latest ?? null;
  const goal = summary.goal ?? null;

  if (!latest && !goal) {
    return (
      <EmptyState
        title="No weight recorded"
        description="Add a weigh-in to start, then set a goal to see when you would reach it."
        action={
          <Button variant="outlined" onClick={onSetGoal}>
            Set a goal
          </Button>
        }
      />
    );
  }

  return (
    <Stack spacing={2}>
      <Stack
        direction="row"
        sx={{ alignItems: 'flex-start', justifyContent: 'space-between', gap: 1 }}
      >
        <Stack spacing={0.25}>
          <Typography variant="caption" color="text.secondary">
            {latest ? `Weighed ${formatDate(latest.recorded_on)}` : 'No weigh-ins yet'}
          </Typography>
          <Typography className="numeral" variant="h2">
            {latest ? formatWeight(latest.weight_kg, display) : '—'}
          </Typography>
        </Stack>
        <Button variant="outlined" onClick={onSetGoal}>
          {goal ? 'Edit goal' : 'Set a goal'}
        </Button>
      </Stack>

      {goal ? (
        <Stack direction="row" spacing={4} sx={{ flexWrap: 'wrap', gap: 2 }}>
          <Fact
            label="Target"
            value={
              goal.target_weight_kg != null
                ? formatWeight(goal.target_weight_kg, display)
                : formatWeight(goal.starting_weight_kg, display)
            }
          />
          <Fact
            label="Since you started"
            value={
              summary.change_since_start_kg != null
                ? formatWeightChange(summary.change_since_start_kg, display)
                : '—'
            }
          />
          <Fact label="At this rate" value={projectionText(summary.projection, goal)} />
          {goal.planned_rate_kg_per_week != null ? (
            <Fact label="Planned" value={formatRate(goal.planned_rate_kg_per_week, display)} />
          ) : null}
        </Stack>
      ) : (
        <Typography variant="body2" color="text.secondary">
          Set a goal to see when you would reach it.
        </Typography>
      )}
    </Stack>
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
            {formatWeight(record.weight_kg, display)} on {formatDate(record.recorded_on)} will be
            removed from your trend.
          </Typography>
          {error ? <Alert severity="error">{error}</Alert> : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button color="error" variant="contained" onClick={onConfirm} disabled={remove.isPending}>
          {remove.isPending ? 'Deleting…' : 'Delete'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
