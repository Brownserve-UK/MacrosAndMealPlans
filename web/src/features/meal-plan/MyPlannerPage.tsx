import AddIcon from '@mui/icons-material/AddOutlined';
import WarningIcon from '@mui/icons-material/WarningAmberOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { ApiError, type Amount, type MealPlanEntry, type MealSlot, type PlannerMeal } from '../../api/client';
import {
  useDeleteMealPlanEntry,
  useMealPlanWeek,
  useMeta,
  useOptOutOfMeal,
  useRejoinMeal,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { addDays, defaultDayFor, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { formatAmount } from './format';
import { MealEditorDialog } from './MealEditorDialog';
import { DayWeekNutrition } from './NutritionSummary';
import { labelForSlot, SLOTS } from './slots';
import { WeekNavigator } from './WeekNavigator';

type EditSelection = { key: string; entry: MealPlanEntry | null; slot: MealSlot };

function fullDayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' });
}

function myPortion(entry: MealPlanEntry, componentId: string, memberId: string | null | undefined): Amount | null {
  const allocation = entry.participants
    .find((person) => person.member_id === memberId)
    ?.allocations.find((candidate) => candidate.component_id === componentId);
  return allocation ? (allocation.allocated as unknown as Amount) : null;
}

function statusChip(entry: MealPlanEntry) {
  if (entry.status === 'eaten') return <Chip size="small" color="success" label="Eaten" />;
  if (entry.status === 'not_eaten') return <Chip size="small" label="Not eaten" />;
  if (entry.status === 'partially_resolved') return <Chip size="small" label="Partly recorded" />;
  return null;
}

function MyMealCard({
  entry,
  memberId,
  busy,
  onEdit,
  onDelete,
  onOptOut,
  onJoin,
  onAddFood,
}: {
  entry: MealPlanEntry;
  memberId: string | null | undefined;
  busy: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onOptOut: () => void;
  onJoin: () => void;
  onAddFood: () => void;
}) {
  const household = entry.scope === 'household';
  const iParticipate = entry.participants.some((person) => person.member_id === memberId);
  const iOptedOut = (entry.opted_out ?? []).some((record) => record.member_id === memberId);
  const myPortionResolved = entry.participants
    .find((person) => person.member_id === memberId)
    ?.allocations.some((allocation) => allocation.status !== 'planned') ?? false;
  const canOptOut = household && iParticipate && !myPortionResolved;

  if (household && iOptedOut) {
    return (
      <Paper variant="outlined" sx={{ px: { xs: 2, sm: 2.5 }, py: 2 }}>
        <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap' }}>
          <Box sx={{ minWidth: 0 }}>
            <Chip label="Opted out" size="small" />
            <Typography color="text.secondary" sx={{ mt: 0.75 }}>
              Household meal{entry.planned_time ? ` at ${entry.planned_time}` : ''}
            </Typography>
          </Box>
          <Stack direction="row" spacing={1}>
            <Button size="small" disabled={busy} onClick={onJoin}>Join meal</Button>
            <Button size="small" startIcon={<AddIcon />} onClick={onAddFood}>Add food</Button>
          </Stack>
        </Stack>
      </Paper>
    );
  }

  return (
    <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
      <Box sx={{ px: { xs: 2, sm: 2.5 }, py: 1.5 }}>
        <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
            {entry.planned_time ? <Typography sx={{ fontWeight: 700 }}>{entry.planned_time}</Typography> : null}
            {household ? <Chip size="small" variant="outlined" label="Household" /> : null}
          </Stack>
          {statusChip(entry)}
        </Stack>
      </Box>
      <Divider />
      <Stack divider={<Divider flexItem />}>
        {entry.components.map((component) => {
          const portion = household ? myPortion(entry, component.id, memberId) : null;
          return (
            <Stack key={component.id} direction="row" spacing={2} sx={{ justifyContent: 'space-between', px: { xs: 2, sm: 2.5 }, py: 1.5 }}>
              <Typography>{component.item_name}</Typography>
              <Typography color="text.secondary" sx={{ whiteSpace: 'nowrap' }}>
                {formatAmount((portion ?? component.amount) as Amount)}
              </Typography>
            </Stack>
          );
        })}
      </Stack>
      {entry.needs_attention ? (
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', px: { xs: 2, sm: 2.5 }, py: 1.25, color: 'warning.dark', bgcolor: 'warning.50' }}>
          <WarningIcon fontSize="small" />
          <Typography variant="body2">Some items need attention</Typography>
        </Stack>
      ) : null}
      <Stack direction="row" spacing={1} sx={{ px: { xs: 1.5, sm: 2 }, py: 1.25, flexWrap: 'wrap' }}>
        {household ? (
          canOptOut ? <Button size="small" disabled={busy} onClick={onOptOut}>Opt out</Button> : null
        ) : (
          <>
            {entry.status === 'planned' ? <Button size="small" onClick={onEdit}>Edit meal</Button> : null}
            {entry.status === 'planned' ? <Button size="small" color="error" onClick={onDelete}>Delete</Button> : null}
          </>
        )}
      </Stack>
    </Paper>
  );
}

export function MyPlannerPage({ weekStart, day }: { weekStart: string; day: string }) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const memberId = principal?.member_id;
  const week = useMealPlanWeek(weekStart);
  const meta = useMeta();
  const directions = meta.data?.nutrient_directions ?? {};
  const remove = useDeleteMealPlanEntry();
  const optOut = useOptOutOfMeal();
  const rejoin = useRejoinMeal();
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [deleting, setDeleting] = useState<MealPlanEntry | null>(null);
  const [error, setError] = useState<string | null>(null);

  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;
  const days = Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
  const canPlan = activeDate >= addDays(todayIso(), -1);
  const busy = optOut.isPending || rejoin.isPending;

  function goToWeek(start: string) {
    void navigate({ to: '/planner/$weekStart/$day', params: { weekStart: start, day: defaultDayFor(start) } });
  }

  function goToDay(date: string) {
    void navigate({ to: '/planner/$weekStart/$day', params: { weekStart, day: date } });
  }

  async function changeAttendance(entry: MealPlanEntry, join: boolean) {
    try {
      await (join ? rejoin : optOut).mutateAsync({ id: entry.id, revision: entry.revision });
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not update this meal.');
    }
  }

  async function deleteMeal() {
    if (!deleting) return;
    try {
      await remove.mutateAsync({ id: deleting.id, revision: deleting.revision });
      setDeleting(null);
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not delete this meal.');
    }
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  const selectedDay = week.data?.days.find((candidate) => candidate.date === activeDate) ?? week.data?.days[0];

  return (
    <Box>
      <PageHeader
        title="My planner"
        actions={canPlan ? <Button variant="contained" startIcon={<AddIcon />} onClick={() => setEditing({ key: crypto.randomUUID(), entry: null, slot: 'dinner' })}>Plan meal</Button> : null}
      />
      {error ? <Alert severity="error" onClose={() => setError(null)} sx={{ mb: 2 }}>{error}</Alert> : null}
      {week.data ? (
        <WeekNavigator
          weekStart={weekStart}
          days={days.map((date) => ({
            date,
            itemCount:
              week.data?.days.find((candidate) => candidate.date === date)?.entries.reduce(
                (sum, entry) => sum + entry.components.length,
                0,
              ) ?? 0,
          }))}
          selectedDate={activeDate}
          currentMonday={startOfWeekIso(todayIso())}
          onWeekChange={goToWeek}
          onDayChange={goToDay}
        />
      ) : null}

      <Typography variant="h2" sx={{ mb: 2 }}>{fullDayLabel(activeDate)}</Typography>
      {week.isLoading ? <Loading label="Loading planner" /> : null}
      {week.data && selectedDay ? (
        <Stack spacing={3}>
          <DayWeekNutrition
            directions={directions}
            day={{
              actual: selectedDay.actual,
              remaining: selectedDay.remaining_planned,
              projected: selectedDay.projected,
              target: selectedDay.target,
            }}
            week={{
              actual: week.data.actual,
              remaining: week.data.remaining_planned,
              projected: week.data.projected,
              target: week.data.target,
              notEnoughData: week.data.insufficient_target_coverage,
            }}
          />
          {SLOTS.map((slot) => {
            const slotEntries = selectedDay.entries.filter((entry) => entry.slot === slot.value);
            const slotHeldByHousehold = slotEntries.some(
              (entry) => entry.scope === 'household' && !(entry.opted_out ?? []).some((record) => record.member_id === memberId),
            );
            return (
              <Box component="section" key={slot.value} aria-labelledby={`planner-${slot.value}`}>
                <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
                  <Typography variant="h3" id={`planner-${slot.value}`}>{labelForSlot(slot.value)}</Typography>
                  {canPlan && !slotHeldByHousehold ? (
                    <Button size="small" startIcon={<AddIcon />} onClick={() => setEditing({ key: crypto.randomUUID(), entry: null, slot: slot.value })}>Add meal</Button>
                  ) : null}
                </Stack>
                {slotEntries.length > 0 ? (
                  <Stack spacing={1.5}>
                    {slotEntries.map((entry) => (
                      <MyMealCard
                        key={entry.id}
                        entry={entry}
                        memberId={memberId}
                        busy={busy}
                        onEdit={() => setEditing({ key: crypto.randomUUID(), entry, slot: entry.slot })}
                        onDelete={() => setDeleting(entry)}
                        onOptOut={() => void changeAttendance(entry, false)}
                        onJoin={() => void changeAttendance(entry, true)}
                        onAddFood={() => setEditing({ key: crypto.randomUUID(), entry: null, slot: entry.slot })}
                      />
                    ))}
                  </Stack>
                ) : <Typography variant="body2" color="text.secondary">No meal planned</Typography>}
              </Box>
            );
          })}
        </Stack>
      ) : null}

      {editing ? <MealEditorDialog key={editing.key} open mode="member" onClose={() => setEditing(null)} date={activeDate} slot={editing.slot} meal={editing.entry ? entryToPlannerMeal(editing.entry) : null} /> : null}
      <Dialog open={Boolean(deleting)} onClose={remove.isPending ? undefined : () => setDeleting(null)}>
        <DialogTitle>Delete this meal?</DialogTitle>
        <DialogContent><Typography>The meal and its planned food will be removed.</Typography></DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleting(null)} disabled={remove.isPending}>Cancel</Button>
          <Button color="error" variant="contained" onClick={() => void deleteMeal()} disabled={remove.isPending}>Delete meal</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

function entryToPlannerMeal(entry: MealPlanEntry): PlannerMeal {
  return {
    id: entry.id,
    scope: entry.scope,
    member_id: entry.member_id ?? undefined,
    owner_name: undefined,
    planned_on: entry.planned_on,
    planned_time: entry.planned_time ?? undefined,
    slot: entry.slot,
    portioning: entry.portioning,
    status: entry.status,
    foods: entry.components.map((component) => ({
      id: component.id,
      ...(component.item_kind === 'recipe'
        ? { item_kind: 'recipe' as const, recipe_id: component.recipe_id }
        : { item_kind: 'product' as const, product_id: component.product_id }),
      item_name: component.item_name,
      amount: component.amount,
      shortage: component.preparation.shortage,
    })),
    people: entry.participants.map((person) => ({
      member_id: person.member_id,
      display_name: person.display_name,
      status: person.status,
      allocations: person.allocations,
      can_record: false,
    })),
    guest_groups: entry.guest_groups,
    opted_out: entry.opted_out ?? [],
    can_opt_out: false,
    can_join: false,
    capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
    revision: entry.revision,
  };
}
