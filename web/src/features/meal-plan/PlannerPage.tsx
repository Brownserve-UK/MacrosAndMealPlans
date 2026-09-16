import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { ApiError, type MealSlot, type PlannerMeal } from '../../api/client';
import { useDeleteMealPlanEntry, useHouseholdPlannerWeek, useOptOutOfMeal, useRejoinMeal } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { CompactNutrition } from './CompactNutrition';
import { addDays, todayIso } from './date';
import { GuidedMealDialog } from './GuidedMealDialog';
import { DeleteMealDialog } from './DeleteMealDialog';
import { MealEditorDialog } from './MealEditorDialog';
import { MealRow, OtherMealsRoster, mealTitle, plannerMealRow } from './MealRow';
import { MealSheet, shortageWarning } from './MealSheet';
import { MealSlotMenu } from './MealSlotMenu';
import { PlannerShell } from './PlannerShell';
import { EmptySlot, SlotSection } from './SlotSection';
import { labelForSlot, MAIN_SLOTS, SLOTS } from './slots';

type EditSelection = { key: string; meal: PlannerMeal | null; slot: MealSlot };

export function PlannerPage({ weekStart, day }: { weekStart: string; day: string }) {
  const { principal } = useAuth();
  const navigate = useNavigate();
  const week = useHouseholdPlannerWeek(weekStart);
  const leave = useOptOutOfMeal();
  const join = useRejoinMeal();
  const remove = useDeleteMealPlanEntry();
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [viewing, setViewing] = useState<PlannerMeal | null>(null);
  const [leaving, setLeaving] = useState<PlannerMeal | null>(null);
  const [deleting, setDeleting] = useState<PlannerMeal | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timeZone = useHouseholdTimeZone();
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;
  const dates = Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
  const canPlan = activeDate >= addDays(todayIso(timeZone), -1);
  const canCoordinate = principal?.permissions.includes('household:write') ?? false;
  const memberId = principal?.member_id;
  const selectedDay = week.data?.days.find((candidate) => candidate.date === activeDate);
  const dayMeals = week.data?.meals.filter((meal) => meal.planned_on === activeDate) ?? [];

  if (!memberId && !canCoordinate) {
    return (
      <EmptyState
        title="Meal planner unavailable"
        description="Your account is not linked to an active household member."
      />
    );
  }
  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  function openEditor(meal: PlannerMeal | null, slot: MealSlot) {
    setEditing({ key: crypto.randomUUID(), meal, slot });
  }

  async function leaveMeal() {
    if (!leaving) return;
    try {
      await leave.mutateAsync({ id: leaving.id, revision: leaving.revision });
      setLeaving(null);
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not leave this meal.');
    }
  }

  async function joinMeal(meal: PlannerMeal) {
    try {
      await join.mutateAsync({ id: meal.id, revision: meal.revision });
      setViewing(null);
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not join this meal.');
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

  const headerChoices = [...MAIN_SLOTS, { value: 'snacks' as const, label: 'Snack' }];

  return (
    <PlannerShell
      weekStart={weekStart}
      activeDate={activeDate}
      dayCounts={week.data ? dates.map((date) => ({
        date,
        itemCount: week.data.meals
          .filter((meal) => meal.planned_on === date && meal.mine)
          .reduce((sum, meal) => sum + meal.foods.length, 0),
      })) : null}
      headerActions={canPlan ? <MealSlotMenu choices={headerChoices} onSelect={(slot) => openEditor(null, slot)} /> : null}
      nutrition={selectedDay ? (
        <CompactNutrition
          day={selectedDay}
          onClick={() => void navigate({ to: '/food-log/$weekStart/$day', params: { weekStart, day: activeDate } })}
        />
      ) : null}
      error={error}
      onDismissError={() => setError(null)}
    >
      {week.isLoading ? <Loading label="Loading planner" /> : null}
      {week.data && selectedDay ? (
        <Stack spacing={3}>
          {SLOTS.map((slot) => {
            const slotMeals = dayMeals.filter((meal) => meal.slot === slot.value);
            const mine = slotMeals.filter((meal) => meal.mine);
            const others = canCoordinate ? slotMeals.filter((meal) => !meal.mine) : [];
            return (
              <SlotSection key={slot.value} id={slot.value} title={labelForSlot(slot.value)}>
                <Stack spacing={1.5}>
                  {mine.length > 0 ? mine.map((meal) => (
                    <MealRow
                      key={meal.id}
                      model={plannerMealRow(meal, memberId)}
                      onClick={() => setViewing(meal)}
                      warning={shortageWarning(meal)}
                    />
                  )) : canPlan ? (
                    <EmptySlot label={`Plan ${slot.label.toLowerCase()}`} onClick={() => openEditor(null, slot.value)} />
                  ) : (
                    <Typography variant="body2" color="text.secondary">No meal planned</Typography>
                  )}
                  {others.length > 0 ? (
                    <Stack spacing={0.75}>
                      <Typography variant="overline" color="text.secondary">{`Also in ${slot.label.toLowerCase()}`}</Typography>
                      <OtherMealsRoster meals={others} onSelect={setViewing} />
                    </Stack>
                  ) : null}
                </Stack>
              </SlotSection>
            );
          })}
        </Stack>
      ) : null}

      {editing ? (
        editing.meal ? (
          <MealEditorDialog
            key={editing.key}
            open
            mode={editing.meal.scope === 'household' || !memberId ? 'household' : 'member'}
            onClose={() => setEditing(null)}
            date={activeDate}
            slot={editing.slot}
            meal={editing.meal}
          />
        ) : (
          <GuidedMealDialog
            key={editing.key}
            open
            onClose={() => setEditing(null)}
            date={activeDate}
            slot={editing.slot}
          />
        )
      ) : null}

      {viewing ? (
        <MealSheet
          meal={viewing}
          onClose={() => setViewing(null)}
          onEdit={() => {
            const meal = viewing;
            setViewing(null);
            openEditor(meal, meal.slot);
          }}
          onDelete={() => {
            setDeleting(viewing);
            setViewing(null);
          }}
          onLeave={() => {
            setLeaving(viewing);
            setViewing(null);
          }}
          onJoin={() => void joinMeal(viewing)}
          busy={join.isPending}
        />
      ) : null}

      <FormDialog open={Boolean(leaving)} onClose={leave.isPending ? undefined : () => setLeaving(null)} fullWidth maxWidth="xs">
        <DialogTitle>Leave this meal?</DialogTitle>
        <DialogContent>
          {leaving ? <Typography>{mealTitle(leaving)}</Typography> : null}
          {error ? <Alert severity="error" sx={{ mt: 2 }}>{error}</Alert> : null}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setLeaving(null)} disabled={leave.isPending}>Cancel</Button>
          <Button variant="contained" onClick={() => void leaveMeal()} disabled={leave.isPending}>Leave meal</Button>
        </DialogActions>
      </FormDialog>

      <DeleteMealDialog
        open={Boolean(deleting)}
        description={deleting ? mealTitle(deleting) : ''}
        busy={remove.isPending}
        onCancel={() => setDeleting(null)}
        onDelete={() => void deleteMeal()}
      />
    </PlannerShell>
  );
}
