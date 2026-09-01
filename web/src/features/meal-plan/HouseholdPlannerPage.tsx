import AddIcon from '@mui/icons-material/AddOutlined';
import ClockIcon from '@mui/icons-material/AccessTimeOutlined';
import PeopleIcon from '@mui/icons-material/PeopleOutlineOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
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
import { Fact, FactBar, MealCard } from './MealCard';
import { MealEditorDialog } from './MealEditorDialog';
import { MealOutcomeDialog } from './MealOutcomeDialog';
import { EmptySlot, SlotSection } from './SlotSection';
import { labelForSlot, MAIN_SLOTS } from './slots';
import { WeekNavigator } from './WeekNavigator';

type EditSelection = { key: string; meal: PlannerMeal | null; slot: MealSlot };

function fullDayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' });
}

function preparationLine(meal: PlannerMeal): string | null {
  const diners = meal.people.length + meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
  if (diners === 0) return null;
  const parts = meal.foods.map((food) => `${food.item_name}: ${formatAmount(food.amount)} for ${diners}`);
  return parts.length > 0 ? parts.join(' · ') : null;
}

function HouseholdMealCard({
  meal,
  onEdit,
  onReview,
  onDelete,
}: {
  meal: PlannerMeal;
  onEdit: () => void;
  onReview: () => void;
  onDelete: () => void;
}) {
  const guests = meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
  const diners = meal.people.length + guests;
  const shortages = meal.foods.filter((food) => food.shortage);
  const line = preparationLine(meal);
  const canReview = meal.people.some((person) => person.can_record && person.allocations.some((a) => a.status === 'planned'))
    || (meal.capabilities.can_record_guests && meal.guest_groups.some((group) => group.allocations.some((a) => a.status === 'planned')));

  return (
    <MealCard
      header={
        <Stack spacing={1}>
          <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
            <FactBar>
              {meal.planned_time ? <Fact icon={<ClockIcon fontSize="small" />} label="Time" value={meal.planned_time} /> : null}
              <Fact icon={<PeopleIcon fontSize="small" />} label="Eating" value={diners === 1 ? '1 person' : `${diners} people`} />
            </FactBar>
            {meal.status === 'eaten' ? <Chip size="small" color="success" label="Recorded" /> : null}
            {meal.status === 'partially_resolved' ? <Chip size="small" label="Partly recorded" /> : null}
          </Stack>
          {(meal.people.length > 0 || meal.opted_out.length > 0 || guests > 0) ? (
            <Stack direction="row" sx={{ flexWrap: 'wrap', gap: 0.5 }}>
              {meal.people.map((person) => (
                <Chip
                  key={person.member_id}
                  size="small"
                  variant="outlined"
                  label={`${person.display_name}${person.status === 'not_eaten' ? ' · did not eat' : ''}`}
                />
              ))}
              {meal.opted_out.map((record) => (
                <Chip key={record.member_id} size="small" variant="outlined" label="Opted out" />
              ))}
              {guests > 0 ? <Chip size="small" variant="outlined" label={guests === 1 ? '1 guest' : `${guests} guests`} /> : null}
            </Stack>
          ) : null}
          {line ? <Typography variant="body2" color="text.secondary">{line}</Typography> : null}
        </Stack>
      }
      foods={meal.foods.map((food) => ({ id: food.id, name: food.item_name, amount: food.amount }))}
      warning={shortages.length > 0 ? `Not enough servings for ${shortages.map((food) => food.item_name).join(', ')}` : null}
      actions={
        <>
          {canReview ? <Button variant="contained" size="small" onClick={onReview}>Review outcomes</Button> : null}
          {meal.capabilities.can_edit ? <Button size="small" onClick={onEdit}>Edit meal</Button> : null}
          {meal.capabilities.can_delete ? <Button size="small" color="error" onClick={onDelete}>Delete</Button> : null}
        </>
      }
    />
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

  function openEditor(meal: PlannerMeal | null, slot: MealSlot) {
    setEditing({ key: crypto.randomUUID(), meal, slot });
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
        actions={canPlan ? <Button variant="contained" startIcon={<AddIcon />} onClick={() => openEditor(null, 'dinner')}>Plan meal</Button> : null}
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
          {MAIN_SLOTS.map((slot) => {
            const slotMeals = meals.filter((meal) => meal.slot === slot.value);
            return (
              <SlotSection
                key={slot.value}
                id={slot.value}
                title={labelForSlot(slot.value)}
                action={
                  canPlan && slotMeals.length > 0
                    ? <Button size="small" startIcon={<AddIcon />} onClick={() => openEditor(null, slot.value)}>Add meal</Button>
                    : null
                }
              >
                {slotMeals.length > 0 ? (
                  <Stack spacing={1.5}>
                    {slotMeals.map((meal) => (
                      <HouseholdMealCard
                        key={meal.id}
                        meal={meal}
                        onEdit={() => openEditor(meal, meal.slot)}
                        onReview={() => setOutcome(meal)}
                        onDelete={() => setDeleting(meal)}
                      />
                    ))}
                  </Stack>
                ) : canPlan ? (
                  <EmptySlot label={`Plan ${slot.label.toLowerCase()}`} onClick={() => openEditor(null, slot.value)} />
                ) : (
                  <Typography variant="body2" color="text.secondary">No meal planned</Typography>
                )}
              </SlotSection>
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
