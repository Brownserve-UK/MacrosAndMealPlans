import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type MealSlot, type PlannerMeal } from '../../api/client';
import { useDeleteMealPlanEntry, useHouseholdPlannerWeek } from '../../api/queries';
import { ErrorState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { addDays, todayIso } from './date';
import { DeleteMealDialog } from './DeleteMealDialog';
import { MealRow, plannerMealRow, type MealAction } from './MealRow';
import { PlannerShell } from './PlannerShell';
import { UseItUp, type PlannableDish } from './UseItUp';
import { CookDialog } from './CookDialog';
import { MealEditorDialog } from './MealEditorDialog';
import { MealOutcomeDialog } from './MealOutcomeDialog';
import { MealSlotMenu } from './MealSlotMenu';
import { EmptySlot, SlotSection } from './SlotSection';
import { labelForSlot, MAIN_SLOTS } from './slots';

type EditSelection = {
  key: string;
  meal: PlannerMeal | null;
  slot: MealSlot;
  dish?: PlannableDish;
};

function HouseholdMealCard({
  meal,
  onEdit,
  onReview,
  onCook,
  onDelete,
}: {
  meal: PlannerMeal;
  onEdit: () => void;
  onReview: () => void;
  onCook: () => void;
  onDelete: () => void;
}) {
  const shortages = meal.foods.filter((food) => food.shortage);
  const canReview = meal.people.some((person) => person.can_record && person.allocations.some((a) => a.status === 'planned'))
    || (meal.capabilities.can_record_guests && meal.guest_groups.some((group) => group.allocations.some((a) => a.status === 'planned')));
  const needsCooking = meal.status !== 'eaten' && meal.foods.some((food) => food.needs_cooking);

  const extras: MealAction[] = [
    ...(meal.capabilities.can_edit ? [{ label: 'Edit meal', onClick: onEdit }] : []),
    ...(meal.capabilities.can_delete ? [{ label: 'Delete meal', onClick: onDelete }] : []),
  ];

  return (
    <MealRow
      model={plannerMealRow(meal)}
      primary={
        needsCooking
          ? { label: 'Cooked it', onClick: onCook }
          : canReview
            ? { label: 'Record meal', onClick: onReview }
            : null
      }
      extras={extras}
      warning={
        shortages.length > 0
          ? `Not enough servings for ${shortages.map((food) => food.item_name).join(', ')}`
          : null
      }
    />
  );
}

export function HouseholdLens({
  weekStart,
  day,
  showLens = true,
  onLensChange = () => undefined,
  enabled = true,
}: {
  weekStart: string;
  day: string;
  showLens?: boolean;
  onLensChange?: (lens: 'mine' | 'household') => void;
  enabled?: boolean;
}) {
  const week = useHouseholdPlannerWeek(weekStart, enabled);
  const remove = useDeleteMealPlanEntry();
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [outcome, setOutcome] = useState<PlannerMeal | null>(null);
  const [deleting, setDeleting] = useState<PlannerMeal | null>(null);
  const [cooking, setCooking] = useState<PlannerMeal | null>(null);
  const [error, setError] = useState<string | null>(null);

  const timeZone = useHouseholdTimeZone();
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;
  const days = Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
  const meals = week.data?.meals.filter((meal) => meal.planned_on === activeDate) ?? [];
  const canPlan = activeDate >= addDays(todayIso(timeZone), -1);

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
    <PlannerShell
      lens="household"
      showLens={showLens}
      onLensChange={onLensChange}
      weekStart={weekStart}
      activeDate={activeDate}
      dayCounts={week.data ? days.map((date) => ({
            date,
            itemCount: (week.data?.meals ?? [])
              .filter((meal) => meal.planned_on === date)
              .reduce((sum, meal) => sum + meal.foods.length, 0),
          })) : null}
      headerActions={canPlan ? <MealSlotMenu choices={MAIN_SLOTS} onSelect={(slot) => openEditor(null, slot)} /> : null}
      error={error}
      onDismissError={() => setError(null)}
    >

      <UseItUp
        today={todayIso(timeZone)}
        onPlan={(dish, slot) => setEditing({ key: crypto.randomUUID(), meal: null, slot, dish })}
      />

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
                        onCook={() => setCooking(meal)}
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

      {editing ? <MealEditorDialog key={editing.key} open mode="household" onClose={() => setEditing(null)} date={activeDate} slot={editing.slot} meal={editing.meal} startWith={editing.dish} /> : null}
      {outcome ? <MealOutcomeDialog meal={outcome} onClose={() => setOutcome(null)} /> : null}
      {cooking ? <CookDialog meal={cooking} onClose={() => setCooking(null)} /> : null}
      <DeleteMealDialog
        open={Boolean(deleting)}
        description="The meal and its attendance plan will be removed."
        busy={remove.isPending}
        onCancel={() => setDeleting(null)}
        onDelete={() => void deleteMeal()}
      />
    </PlannerShell>
  );
}
