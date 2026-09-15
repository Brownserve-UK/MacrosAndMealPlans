import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState, type ReactNode } from 'react';
import { ApiError, type MealPlanEntry, type MealSlot } from '../../api/client';
import {
  useDeleteMealPlanEntry,
  useMealPlanWeek,
  useMeta,
  useOptOutOfMeal,
  useRejoinMeal,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { ErrorState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { addDays, todayIso } from './date';
import { DeleteMealDialog } from './DeleteMealDialog';
import { MealRow } from './MealRow';
import { MealEditorDialog } from './MealEditorDialog';
import { MealSlotMenu } from './MealSlotMenu';
import { PlannerShell } from './PlannerShell';
import { entryToPlannerMeal } from './plannerMeal';
import { SaveForReuseDialog } from './SaveForReuseDialog';
import { DayWeekNutrition } from './NutritionSummary';
import { EmptySlot, SlotSection } from './SlotSection';
import { SnackSection } from './SnackSection';
import { labelForSlot, MAIN_SLOTS } from './slots';

type EditSelection = { key: string; entry: MealPlanEntry | null; slot: MealSlot };

function iParticipateIn(entry: MealPlanEntry, memberId: string | null | undefined) {
  return entry.participants.some((person) => person.member_id === memberId);
}

function iOptedOutOf(entry: MealPlanEntry, memberId: string | null | undefined) {
  return (entry.opted_out ?? []).some((record) => record.member_id === memberId);
}

function myPortionResolved(entry: MealPlanEntry, memberId: string | null | undefined) {
  return entry.participants
    .find((person) => person.member_id === memberId)
    ?.allocations.some((allocation) => allocation.status !== 'planned') ?? false;
}

function shortageWarning(entry: MealPlanEntry) {
  const shortages = entry.components.filter((component) => component.preparation.shortage);
  return shortages.length > 0
    ? `Not enough servings for ${shortages.map((component) => component.item_name).join(', ')}`
    : null;
}

function OwnMealCard({
  entry,
  canPlan,
  onAddFood,
  onEdit,
  onDelete,
  onSaveForReuse,
}: {
  entry: MealPlanEntry;
  canPlan: boolean;
  onAddFood: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onSaveForReuse: () => void;
}) {
  const kcal = entry.planned.nutrition.energy_kcal;
  const editable = canPlan && (entry.status === 'planned' || entry.status === 'assumed');
  const canSave = entry.components.some((component) => component.item_kind !== 'dish');
  const tagLabel = entry.status === 'eaten'
    ? 'Eaten'
    : entry.status === 'not_eaten'
      ? 'Not eaten'
      : entry.status === 'partially_resolved'
        ? 'Partly recorded'
        : null;

  return (
    <MealRow
      model={{
        time: entry.planned_time,
        title: entry.components.map((component) => component.item_name).join(', ') || 'Nothing planned',
        chips: [],
        detail: kcal != null ? `${Math.round(kcal)} kcal` : null,
        tag: tagLabel ? { label: tagLabel } : null,
      }}
      primary={editable ? { label: 'Add food', onClick: onAddFood } : null}
      extras={editable ? [
        { label: 'Edit meal', onClick: onEdit },
        { label: 'Delete meal', onClick: onDelete },
        ...(canSave ? [{ label: 'Save for reuse', onClick: onSaveForReuse }] : []),
      ] : []}
      warning={shortageWarning(entry) ?? (entry.needs_attention ? 'Some items need attention' : null)}
    />
  );
}

function HouseholdHeldCard({
  entry,
  memberId,
  busy,
  onOptOut,
}: {
  entry: MealPlanEntry;
  memberId: string | null | undefined;
  busy: boolean;
  onOptOut: () => void;
}) {
  const canOptOut =
    !myPortionResolved(entry, memberId)
    && (entry.status === 'planned' || entry.status === 'assumed');
  return (
    <MealRow
      model={{
        time: entry.planned_time,
        title: entry.components.map((component) => component.item_name).join(', ') || 'Nothing planned',
        chips: [{ key: 'household', label: 'Household meal' }],
        detail: null,
        tag: null,
      }}
      secondary={canOptOut && !busy ? { label: 'Opt out to plan your own', onClick: onOptOut } : null}
      warning={shortageWarning(entry)}
    />
  );
}

function OptedOutCard({
  entry,
  busy,
  onJoin,
}: {
  entry: MealPlanEntry;
  busy: boolean;
  onJoin: () => void;
}) {
  return (
    <MealRow
      model={{
        time: entry.planned_time,
        title: 'Household meal',
        chips: [{ key: 'opted-out', label: 'Opted out', muted: true }],
        detail: null,
        tag: null,
      }}
      primary={busy ? null : { label: 'Join meal', onClick: onJoin }}
    />
  );
}

export function MineLens({
  weekStart,
  day,
  showLens = false,
  onLensChange = () => undefined,
  enabled = true,
}: {
  weekStart: string;
  day: string;
  showLens?: boolean;
  onLensChange?: (lens: 'mine' | 'household') => void;
  enabled?: boolean;
}) {
  const { principal } = useAuth();
  const memberId = principal?.member_id;
  const week = useMealPlanWeek(weekStart, enabled);
  const meta = useMeta();
  const directions = meta.data?.nutrient_directions ?? {};
  const remove = useDeleteMealPlanEntry();
  const optOut = useOptOutOfMeal();
  const rejoin = useRejoinMeal();
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [deleting, setDeleting] = useState<MealPlanEntry | null>(null);
  const [saving, setSaving] = useState<MealPlanEntry | null>(null);
  const [error, setError] = useState<string | null>(null);

  const timeZone = useHouseholdTimeZone();
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;
  const days = Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
  const canPlan = activeDate >= addDays(todayIso(timeZone), -1);
  const busy = optOut.isPending || rejoin.isPending;

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

  function openEditor(entry: MealPlanEntry | null, slot: MealSlot) {
    setEditing({ key: crypto.randomUUID(), entry, slot });
  }

  const headerChoices = [
    ...MAIN_SLOTS.filter((slot) => !selectedDay?.entries.some((entry) => (
      entry.slot === slot.value
      && (entry.scope === 'member' || iParticipateIn(entry, memberId))
    ))),
    { value: 'snacks' as const, label: 'Snack' },
  ];

  return (
    <PlannerShell
      lens="mine"
      showLens={showLens}
      onLensChange={onLensChange}
      weekStart={weekStart}
      activeDate={activeDate}
      dayCounts={week.data ? days.map((date) => ({
            date,
            itemCount:
              week.data?.days.find((candidate) => candidate.date === date)?.entries.reduce(
                (sum, entry) => sum + entry.components.length,
                0,
              ) ?? 0,
          })) : null}
      headerActions={canPlan ? <MealSlotMenu choices={headerChoices} onSelect={(slot) => openEditor(null, slot)} /> : null}
      error={error}
      onDismissError={() => setError(null)}
    >
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
              calorieDirection: selectedDay.calorie_direction,
            }}
            week={{
              actual: week.data.actual,
              remaining: week.data.remaining_planned,
              projected: week.data.projected,
              target: week.data.target,
              calorieDirection: week.data.calorie_direction,
              notEnoughData: week.data.insufficient_target_coverage,
            }}
          />
          {MAIN_SLOTS.map((slot) => {
            const slotEntries = selectedDay.entries.filter((entry) => entry.slot === slot.value);
            const ownEntry = slotEntries.find((entry) => entry.scope === 'member');
            const heldEntry = slotEntries.find(
              (entry) => entry.scope === 'household' && iParticipateIn(entry, memberId),
            );
            const optedOutEntry = slotEntries.find(
              (entry) => entry.scope === 'household' && iOptedOutOf(entry, memberId) && !iParticipateIn(entry, memberId),
            );

            let body: ReactNode;
            if (ownEntry) {
              body = (
                <OwnMealCard
                  entry={ownEntry}
                  canPlan={canPlan}
                  onAddFood={() => openEditor(ownEntry, ownEntry.slot)}
                  onEdit={() => openEditor(ownEntry, ownEntry.slot)}
                  onDelete={() => setDeleting(ownEntry)}
                  onSaveForReuse={() => setSaving(ownEntry)}
                />
              );
            } else if (heldEntry) {
              body = (
                <HouseholdHeldCard
                  entry={heldEntry}
                  memberId={memberId}
                  busy={busy}
                  onOptOut={() => void changeAttendance(heldEntry, false)}
                />
              );
            } else if (optedOutEntry) {
              body = (
                <Stack spacing={1.5}>
                  <OptedOutCard entry={optedOutEntry} busy={busy} onJoin={() => void changeAttendance(optedOutEntry, true)} />
                  {canPlan ? <EmptySlot label={`Plan ${slot.label.toLowerCase()}`} onClick={() => openEditor(null, slot.value)} /> : null}
                </Stack>
              );
            } else if (canPlan) {
              body = <EmptySlot label={`Plan ${slot.label.toLowerCase()}`} onClick={() => openEditor(null, slot.value)} />;
            } else {
              body = <Typography variant="body2" color="text.secondary">No meal planned</Typography>;
            }

            return (
              <SlotSection key={slot.value} id={slot.value} title={labelForSlot(slot.value)}>
                {body}
              </SlotSection>
            );
          })}

          <SlotSection id="snacks" title="Snacks">
            <SnackSection
              entries={selectedDay.entries.filter((entry) => entry.slot === 'snacks')}
              memberId={memberId}
              canPlan={canPlan}
              onAddSnack={() => openEditor(null, 'snacks')}
              onAddFood={(entry) => openEditor(entry, 'snacks')}
              onEdit={(entry) => openEditor(entry, 'snacks')}
              onDelete={(entry) => setDeleting(entry)}
            />
          </SlotSection>
        </Stack>
      ) : null}

      {editing ? (
        <MealEditorDialog
          key={editing.key}
          open
          mode="member"
          onClose={() => setEditing(null)}
          date={activeDate}
          slot={editing.slot}
          meal={editing.entry ? entryToPlannerMeal(editing.entry, {
            canRecord: false,
            capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
          }) : null}
        />
      ) : null}
      <DeleteMealDialog
        open={Boolean(deleting)}
        description="The meal and its planned food will be removed."
        busy={remove.isPending}
        onCancel={() => setDeleting(null)}
        onDelete={() => void deleteMeal()}
      />
      {saving ? (
        <SaveForReuseDialog open onClose={() => setSaving(null)} entry={saving} />
      ) : null}
    </PlannerShell>
  );
}
