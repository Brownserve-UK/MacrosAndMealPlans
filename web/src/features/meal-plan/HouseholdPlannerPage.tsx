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
import { useDeleteMealPlanEntry, useHouseholdPlannerWeek } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { addDays, defaultDayFor, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { formatAmount } from './format';
import { MealEditorDialog } from './MealEditorDialog';
import { MealOutcomeDialog } from './MealOutcomeDialog';
import { labelForSlot } from './slots';
import { WeekNavigator } from './WeekNavigator';

const HOUSEHOLD_SLOTS: { value: MealSlot; label: string }[] = [
  { value: 'breakfast', label: 'Breakfast' },
  { value: 'lunch', label: 'Lunch' },
  { value: 'dinner', label: 'Dinner' },
];

type EditSelection = { key: string; meal: PlannerMeal | null; slot: MealSlot };

function fullDayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' });
}

function preparationLine(meal: PlannerMeal): string | null {
  const parts: string[] = [];
  for (const food of meal.foods) {
    const prep = meal.people.length + meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
    if (prep === 0) continue;
    parts.push(`${food.item_name}: ${formatAmount(food.amount)} for ${prep}`);
  }
  return parts.length > 0 ? parts.join(' · ') : null;
}

function HouseholdMealCard({
  meal,
  onEdit,
  onPortions,
  onReview,
  onDelete,
}: {
  meal: PlannerMeal;
  onEdit: () => void;
  onPortions: () => void;
  onReview: () => void;
  onDelete: () => void;
}) {
  const guests = meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
  const shortages = meal.foods.filter((food) => food.shortage);
  const line = preparationLine(meal);
  const canReview = meal.people.some((person) => person.can_record && person.allocations.some((a) => a.status === 'planned'))
    || (meal.capabilities.can_record_guests && meal.guest_groups.some((group) => group.allocations.some((a) => a.status === 'planned')));

  return (
    <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
      <Box sx={{ px: { xs: 2, sm: 2.5 }, py: 2 }}>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexWrap: 'wrap', mb: 1 }}>
          {meal.planned_time ? <Typography sx={{ fontWeight: 700 }}>{meal.planned_time}</Typography> : null}
          {meal.status === 'eaten' ? <Chip size="small" color="success" label="Recorded" /> : null}
          {meal.status === 'partially_resolved' ? <Chip size="small" label="Partly recorded" /> : null}
        </Stack>
        <Stack direction="row" sx={{ flexWrap: 'wrap', gap: 0.5 }}>
          {meal.people.map((person) => (
            <Chip key={person.member_id} size="small" variant="outlined" label={`${person.display_name}${person.status === 'not_eaten' ? ' · did not eat' : ''}`} />
          ))}
          {meal.opted_out.map((record) => (
            <Chip key={record.member_id} size="small" variant="outlined" color="default" label="Opted out" />
          ))}
          {guests > 0 ? <Chip size="small" variant="outlined" label={guests === 1 ? '1 guest' : `${guests} guests`} /> : null}
        </Stack>
        {line ? <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>{line}</Typography> : null}
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
          <Typography variant="body2">Not enough servings for {shortages.map((food) => food.item_name).join(', ')}</Typography>
        </Stack>
      ) : null}
      <Stack direction="row" spacing={1} sx={{ px: { xs: 1.5, sm: 2 }, py: 1.25, flexWrap: 'wrap' }}>
        {canReview ? <Button variant="contained" size="small" onClick={onReview}>Review outcomes</Button> : null}
        {meal.capabilities.can_edit ? <Button size="small" onClick={onEdit}>Edit meal</Button> : null}
        {meal.capabilities.can_edit ? <Button size="small" onClick={onPortions}>Portions</Button> : null}
        {meal.capabilities.can_delete ? <Button size="small" color="error" onClick={onDelete}>Delete</Button> : null}
      </Stack>
    </Paper>
  );
}

export function HouseholdPlannerPage({ weekStart, day }: { weekStart: string; day: string }) {
  const navigate = useNavigate();
  const week = useHouseholdPlannerWeek(weekStart);
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
    void navigate({ to: '/household/planner/$weekStart/$day', params: { weekStart: start, day: defaultDayFor(start) } });
  }

  function goToDay(date: string) {
    void navigate({ to: '/household/planner/$weekStart/$day', params: { weekStart, day: date } });
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

  return (
    <Box>
      <PageHeader
        title="Household planner"
        actions={canPlan ? <Button variant="contained" startIcon={<AddIcon />} onClick={() => setEditing({ key: crypto.randomUUID(), meal: null, slot: 'dinner' })}>Plan meal</Button> : null}
      />
      {error ? <Alert severity="error" onClose={() => setError(null)} sx={{ mb: 2 }}>{error}</Alert> : null}
      {week.data ? (
        <WeekNavigator
          weekStart={weekStart}
          days={days.map((date) => ({
            date,
            itemCount: (week.data?.meals ?? [])
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
      {week.isLoading ? <Loading label="Loading household planner" /> : null}
      {week.data ? (
        <Stack spacing={3}>
          {HOUSEHOLD_SLOTS.map((slot) => {
            const slotMeals = meals.filter((meal) => meal.slot === slot.value);
            return (
              <Box component="section" key={slot.value} aria-labelledby={`household-${slot.value}`}>
                <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
                  <Typography variant="h3" id={`household-${slot.value}`}>{labelForSlot(slot.value)}</Typography>
                  {canPlan ? <Button size="small" startIcon={<AddIcon />} onClick={() => setEditing({ key: crypto.randomUUID(), meal: null, slot: slot.value })}>Add meal</Button> : null}
                </Stack>
                {slotMeals.length > 0 ? (
                  <Stack spacing={1.5}>
                    {slotMeals.map((meal) => (
                      <HouseholdMealCard
                        key={meal.id}
                        meal={meal}
                        onEdit={() => setEditing({ key: crypto.randomUUID(), meal, slot: meal.slot })}
                        onPortions={() => setEditing({ key: crypto.randomUUID(), meal, slot: meal.slot })}
                        onReview={() => setOutcome(meal)}
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

      {editing ? <MealEditorDialog key={editing.key} open mode="household" onClose={() => setEditing(null)} date={activeDate} slot={editing.slot} meal={editing.meal} /> : null}
      {outcome ? <MealOutcomeDialog meal={outcome} onClose={() => setOutcome(null)} /> : null}
      <Dialog open={Boolean(deleting)} onClose={remove.isPending ? undefined : () => setDeleting(null)}>
        <DialogTitle>Delete this meal?</DialogTitle>
        <DialogContent><Typography>The meal and its attendance plan will be removed.</Typography></DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleting(null)} disabled={remove.isPending}>Cancel</Button>
          <Button color="error" variant="contained" onClick={() => void deleteMeal()} disabled={remove.isPending}>Delete meal</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
