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
import { ApiError, type MealSlot, type PlannerMeal } from '../../api/client';
import { useDeleteMealPlanEntry, usePlannerWeek } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { addDays, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { defaultDayFor } from './MealPlanPage';
import { MealEditorDialog } from './MealEditorDialog';
import { MealOutcomeDialog } from './MealOutcomeDialog';
import { formatAmount } from './format';
import { labelForSlot, SLOTS } from './slots';
import { WeekNavigator } from './WeekNavigator';

type EditSelection = { key: string; meal: PlannerMeal | null; slot: MealSlot };

function fullDayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' });
}

function canRecordMeal(meal: PlannerMeal, future: boolean) {
  return !future && (
    meal.people.some((person) => person.can_record && person.allocations.some((allocation) => allocation.status === 'planned'))
    || (meal.capabilities.can_record_guests && meal.guest_groups.some((group) => group.allocations.some((allocation) => allocation.status === 'planned')))
  );
}

function attendance(meal: PlannerMeal) {
  const names = meal.people.map((person) => person.display_name);
  const guests = meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
  const parts = [...names];
  if (guests > 0) parts.push(guests === 1 ? '1 guest' : `${guests} guests`);
  if (parts.length <= 1) return parts[0] ?? 'No diners selected';
  return `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}`;
}

function statusLabel(meal: PlannerMeal) {
  if (meal.status === 'eaten') return 'Recorded';
  if (meal.status === 'not_eaten') return 'Not eaten';
  if (meal.status === 'partially_resolved') return 'Partly recorded';
  return null;
}

function mealProblems(meal: PlannerMeal) {
  const problems = meal.people.flatMap((person) => {
    if (person.status === 'not_eaten') return [`${person.display_name} did not eat`];
    if (person.status === 'partially_resolved') return [`${person.display_name}'s meal differed`];
    if (person.status === 'planned' && meal.status !== 'planned') return [`Waiting for ${person.display_name}`];
    return [];
  });
  for (const group of meal.guest_groups) {
    const label = group.count === 1 ? 'Guest' : `${group.count} guests`;
    if (group.status === 'not_eaten') problems.push(`${label} did not eat`);
    if (group.status === 'partially_resolved') problems.push(`${label} differed`);
    if (group.status === 'planned' && meal.status !== 'planned') problems.push(`Waiting for ${label.toLowerCase()}`);
  }
  return problems;
}

function MealCard({
  meal,
  future,
  showOwner,
  onEdit,
  onOutcome,
  onDelete,
}: {
  meal: PlannerMeal;
  future: boolean;
  showOwner: boolean;
  onEdit: () => void;
  onOutcome: () => void;
  onDelete: () => void;
}) {
  const status = statusLabel(meal);
  const canRecord = canRecordMeal(meal, future);
  const shortages = meal.foods.filter((food) => food.shortage);
  const problems = mealProblems(meal);

  return (
    <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
      <Box sx={{ px: { xs: 2, sm: 2.5 }, py: 2 }}>
        <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'flex-start' }}>
          <Box sx={{ minWidth: 0 }}>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
              {meal.planned_time ? <Typography sx={{ fontWeight: 700 }}>{meal.planned_time}</Typography> : null}
              {showOwner && meal.owner_name ? <Typography color="text.secondary">For {meal.owner_name}</Typography> : null}
            </Stack>
            {meal.scope === 'household' ? <Typography color="text.secondary" sx={{ mt: 0.5 }}>{attendance(meal)}</Typography> : null}
            {problems.length > 0 ? <Typography variant="body2" color="warning.dark" sx={{ mt: 0.75 }}>{problems.join(' · ')}</Typography> : null}
          </Box>
          {status ? <Chip label={status} size="small" color={meal.status === 'eaten' ? 'success' : 'default'} /> : null}
        </Stack>
      </Box>
      <Divider />
      <Stack divider={<Divider flexItem />}>
        {meal.foods.map((food) => (
          <Stack key={food.id} direction="row" spacing={2} sx={{ justifyContent: 'space-between', px: { xs: 2, sm: 2.5 }, py: 1.5 }}>
            <Typography>{food.item_name}</Typography>
            <Typography color="text.secondary" sx={{ whiteSpace: 'nowrap' }}>{formatAmount(food.amount)}</Typography>
          </Stack>
        ))}
      </Stack>
      {shortages.length > 0 ? (
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', px: { xs: 2, sm: 2.5 }, py: 1.25, color: 'warning.dark', bgcolor: 'warning.50' }}>
          <WarningIcon fontSize="small" />
          <Typography variant="body2">Not enough stock for {shortages.map((food) => food.item_name).join(', ')}</Typography>
        </Stack>
      ) : null}
      {(meal.capabilities.can_edit || canRecord || meal.capabilities.can_delete) ? (
        <Stack direction="row" spacing={1} sx={{ px: { xs: 1.5, sm: 2 }, py: 1.25, flexWrap: 'wrap' }}>
          {canRecord ? <Button variant="contained" size="small" onClick={onOutcome}>{meal.status === 'partially_resolved' ? 'Record remaining' : 'Mark as eaten'}</Button> : null}
          {meal.capabilities.can_edit ? <Button size="small" onClick={onEdit}>Edit meal</Button> : null}
          {meal.capabilities.can_delete ? <Button size="small" color="error" onClick={onDelete}>Delete</Button> : null}
        </Stack>
      ) : null}
    </Paper>
  );
}

function SnackList({
  meals,
  future,
  memberId,
  canPlan,
  onAdd,
  onEdit,
  onOutcome,
  onDelete,
}: {
  meals: PlannerMeal[];
  future: boolean;
  memberId: string | null | undefined;
  canPlan: boolean;
  onAdd: () => void;
  onEdit: (meal: PlannerMeal) => void;
  onOutcome: (meal: PlannerMeal) => void;
  onDelete: (meal: PlannerMeal) => void;
}) {
  const ordered = [...meals].sort((left, right) => {
    if (left.planned_time === right.planned_time) return left.id.localeCompare(right.id);
    if (!left.planned_time) return 1;
    if (!right.planned_time) return -1;
    return left.planned_time.localeCompare(right.planned_time);
  });

  return (
    <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
      {ordered.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ px: { xs: 2, sm: 2.5 }, py: 2 }}>
          No snacks planned
        </Typography>
      ) : (
        <Stack divider={<Divider flexItem />}>
          {ordered.map((meal) => {
            const status = statusLabel(meal);
            const problems = mealProblems(meal);
            const shortages = meal.foods.filter((food) => food.shortage);
            const showOwner = meal.scope === 'member' && meal.member_id !== memberId;
            const showContext = showOwner || meal.scope === 'household' || Boolean(status) || problems.length > 0;
            const canRecord = canRecordMeal(meal, future);
            const showActions = meal.capabilities.can_edit || canRecord || meal.capabilities.can_delete;
            const snackLabel = meal.planned_time ? `${meal.planned_time} snack` : 'untimed snack';
            return (
              <Box key={meal.id}>
                {showContext ? (
                  <Stack
                    direction="row"
                    spacing={2}
                    sx={{ justifyContent: 'space-between', alignItems: 'flex-start', px: { xs: 2, sm: 2.5 }, py: 1.25, bgcolor: 'action.hover' }}
                  >
                    <Box sx={{ minWidth: 0 }}>
                      {showOwner && meal.owner_name ? <Typography variant="body2">For {meal.owner_name}</Typography> : null}
                      {meal.scope === 'household' ? <Typography variant="body2" color="text.secondary">{attendance(meal)}</Typography> : null}
                      {problems.length > 0 ? <Typography variant="body2" color="warning.dark">{problems.join(' · ')}</Typography> : null}
                    </Box>
                    {status ? <Chip label={status} size="small" color={meal.status === 'eaten' ? 'success' : 'default'} /> : null}
                  </Stack>
                ) : null}
                <Stack divider={<Divider flexItem />}>
                  {meal.foods.map((food) => (
                    <Stack key={food.id} direction="row" spacing={2} sx={{ justifyContent: 'space-between', px: { xs: 2, sm: 2.5 }, py: 1.5 }}>
                      <Box sx={{ minWidth: 0 }}>
                        <Typography>{food.item_name}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {[meal.planned_time, formatAmount(food.amount)].filter(Boolean).join(' · ')}
                        </Typography>
                      </Box>
                    </Stack>
                  ))}
                </Stack>
                {shortages.length > 0 ? (
                  <Stack direction="row" spacing={1} sx={{ alignItems: 'center', px: { xs: 2, sm: 2.5 }, py: 1.25, color: 'warning.dark', bgcolor: 'warning.50' }}>
                    <WarningIcon fontSize="small" />
                    <Typography variant="body2">Not enough stock for {shortages.map((food) => food.item_name).join(', ')}</Typography>
                  </Stack>
                ) : null}
                {showActions ? (
                  <Stack direction="row" spacing={1} sx={{ px: { xs: 1.5, sm: 2 }, py: 1.25, flexWrap: 'wrap' }}>
                    {canRecord ? (
                      <Button
                        variant="contained"
                        size="small"
                        aria-label={meal.status === 'partially_resolved' ? `Record remaining for ${snackLabel}` : `Mark ${snackLabel} as eaten`}
                        onClick={() => onOutcome(meal)}
                      >
                        {meal.status === 'partially_resolved' ? 'Record remaining' : 'Mark as eaten'}
                      </Button>
                    ) : null}
                    {meal.capabilities.can_edit ? <Button size="small" aria-label={`Edit ${snackLabel}`} onClick={() => onEdit(meal)}>Edit snack</Button> : null}
                    {meal.capabilities.can_delete ? <Button size="small" color="error" aria-label={`Delete ${snackLabel}`} onClick={() => onDelete(meal)}>Delete</Button> : null}
                  </Stack>
                ) : null}
              </Box>
            );
          })}
        </Stack>
      )}
      {canPlan ? (
        <Button fullWidth startIcon={<AddIcon />} onClick={onAdd} sx={{ py: 1.25, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}>
          Add snack
        </Button>
      ) : null}
    </Paper>
  );
}

export function PlannerPage({ weekStart, day }: { weekStart: string; day: string }) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const week = usePlannerWeek(weekStart);
  const remove = useDeleteMealPlanEntry();
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [outcome, setOutcome] = useState<PlannerMeal | null>(null);
  const [deleting, setDeleting] = useState<PlannerMeal | null>(null);
  const [error, setError] = useState<string | null>(null);
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;
  const days = Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
  const meals = week.data?.meals.filter((meal) => meal.planned_on === activeDate) ?? [];
  const canPlan = activeDate >= addDays(todayIso(), -1);

  function goToWeek(start: string) {
    void navigate({ to: '/planner/$weekStart/$day', params: { weekStart: start, day: defaultDayFor(start) } });
  }

  function goToDay(date: string) {
    void navigate({ to: '/planner/$weekStart/$day', params: { weekStart, day: date } });
  }

  async function deleteMeal() {
    if (!deleting) return;
    try {
      await remove.mutateAsync({ id: deleting.id, revision: deleting.revision });
      setDeleting(null);
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : `Could not delete this ${deleting.slot === 'snacks' ? 'snack' : 'meal'}.`);
    }
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  return (
    <Box>
      <PageHeader
        title="Planner"
        subtitle="Plan meals and record what happened in one place."
        actions={canPlan ? <Button variant="contained" startIcon={<AddIcon />} onClick={() => setEditing({ key: crypto.randomUUID(), meal: null, slot: 'dinner' })}>Plan meal</Button> : null}
      />
      {error ? <Alert severity="error" onClose={() => setError(null)} sx={{ mb: 2 }}>{error}</Alert> : null}
      {week.data ? (
        <WeekNavigator
          weekStart={weekStart}
          days={days.map((date) => ({
            date,
            itemCount: week.data.meals
              .filter((meal) => meal.planned_on === date)
              .reduce((sum, meal) => sum + meal.foods.length, 0),
          }))}
          selectedDate={activeDate}
          currentMonday={startOfWeekIso(todayIso())}
          onWeekChange={goToWeek}
          onDayChange={goToDay}
        />
      ) : null}

      <Typography variant="h2" sx={{ mb: 2 }}>{fullDayLabel(activeDate)}</Typography>
      {week.isLoading ? <Loading label="Loading planner" /> : null}
      {week.data ? (
        <Stack spacing={3}>
          {SLOTS.map((slot) => {
            const slotMeals = meals.filter((meal) => meal.slot === slot.value);
            return (
              <Box component="section" key={slot.value} aria-labelledby={`planner-${slot.value}`}>
                <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
                  <Typography variant="h3" id={`planner-${slot.value}`}>{labelForSlot(slot.value)}</Typography>
                  {canPlan && slot.value !== 'snacks' ? <Button size="small" startIcon={<AddIcon />} onClick={() => setEditing({ key: crypto.randomUUID(), meal: null, slot: slot.value })}>Add meal</Button> : null}
                </Stack>
                {slot.value === 'snacks' ? (
                  <SnackList
                    meals={slotMeals}
                    future={activeDate > todayIso()}
                    memberId={principal?.member_id}
                    canPlan={canPlan}
                    onAdd={() => setEditing({ key: crypto.randomUUID(), meal: null, slot: 'snacks' })}
                    onEdit={(meal) => setEditing({ key: crypto.randomUUID(), meal, slot: meal.slot })}
                    onOutcome={setOutcome}
                    onDelete={setDeleting}
                  />
                ) : slotMeals.length > 0 ? (
                  <Stack spacing={1.5}>
                    {slotMeals.map((meal) => (
                      <MealCard
                        key={meal.id}
                        meal={meal}
                        future={activeDate > todayIso()}
                        showOwner={meal.scope === 'member' && meal.member_id !== principal?.member_id}
                        onEdit={() => setEditing({ key: crypto.randomUUID(), meal, slot: meal.slot })}
                        onOutcome={() => setOutcome(meal)}
                        onDelete={() => setDeleting(meal)}
                      />
                    ))}
                  </Stack>
                ) : <Typography variant="body2" color="text.secondary">No meal planned</Typography>}
              </Box>
            );
          })}
        </Stack>
      ) : null}

      {editing ? <MealEditorDialog key={editing.key} open onClose={() => setEditing(null)} date={activeDate} slot={editing.slot} meal={editing.meal} /> : null}
      {outcome ? <MealOutcomeDialog meal={outcome} onClose={() => setOutcome(null)} /> : null}
      <Dialog open={Boolean(deleting)} onClose={remove.isPending ? undefined : () => setDeleting(null)}>
        <DialogTitle>Delete this {deleting?.slot === 'snacks' ? 'snack' : 'meal'}?</DialogTitle>
        <DialogContent><Typography>The {deleting?.slot === 'snacks' ? 'snack' : 'meal'} and its attendance plan will be removed.</Typography></DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleting(null)} disabled={remove.isPending}>Cancel</Button>
          <Button color="error" variant="contained" onClick={() => void deleteMeal()} disabled={remove.isPending}>Delete {deleting?.slot === 'snacks' ? 'snack' : 'meal'}</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
