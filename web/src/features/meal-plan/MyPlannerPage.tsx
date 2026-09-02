import AddIcon from '@mui/icons-material/AddOutlined';
import ClockIcon from '@mui/icons-material/AccessTimeOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState, type ReactNode } from 'react';
import { ApiError, type MealPlanEntry, type MealSlot, type PlannerMeal } from '../../api/client';
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
import { Fact, FactBar, MealCard } from './MealCard';
import { MealEditorDialog } from './MealEditorDialog';
import { MealSlotMenu } from './MealSlotMenu';
import { DayWeekNutrition } from './NutritionSummary';
import { EmptySlot, SlotSection } from './SlotSection';
import { SnackSection } from './SnackSection';
import { labelForSlot, MAIN_SLOTS } from './slots';
import { WeekNavigator } from './WeekNavigator';

type EditSelection = { key: string; entry: MealPlanEntry | null; slot: MealSlot };

function fullDayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' });
}

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

function statusChip(entry: MealPlanEntry) {
  if (entry.status === 'eaten') return <Chip size="small" color="success" label="Eaten" />;
  if (entry.status === 'not_eaten') return <Chip size="small" label="Not eaten" />;
  if (entry.status === 'partially_resolved') return <Chip size="small" label="Partly recorded" />;
  if (entry.status === 'assumed') return <Chip size="small" color="warning" variant="outlined" label="Assumed" />;
  return null;
}

function OwnMealCard({
  entry,
  canPlan,
  onAddFood,
  onEdit,
  onDelete,
}: {
  entry: MealPlanEntry;
  canPlan: boolean;
  onAddFood: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const kcal = entry.planned.nutrition.energy_kcal;
  const itemCount = entry.components.length;
  const plannedValue = kcal != null
    ? `${itemCount} ${itemCount === 1 ? 'item' : 'items'} · ${Math.round(kcal)} kcal`
    : `${itemCount} ${itemCount === 1 ? 'item' : 'items'}`;
  const editable = canPlan && (entry.status === 'planned' || entry.status === 'assumed');

  return (
    <MealCard
      header={
        <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <FactBar>
            {entry.planned_time ? <Fact icon={<ClockIcon fontSize="small" />} label="Time" value={entry.planned_time} /> : null}
            <Fact label="Planned" value={plannedValue} />
          </FactBar>
          {statusChip(entry)}
        </Stack>
      }
      foods={entry.components.map((component) => ({ id: component.id, name: component.item_name, amount: component.amount }))}
      warning={entry.needs_attention ? 'Some items need attention' : null}
      actions={
        editable ? (
          <>
            <Button size="small" startIcon={<AddIcon />} onClick={onAddFood}>Add food</Button>
            <Button size="small" onClick={onEdit}>Edit meal</Button>
            <Button size="small" color="error" onClick={onDelete}>Delete</Button>
          </>
        ) : null
      }
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
    <MealCard
      header={
        <FactBar>
          {entry.planned_time ? <Fact icon={<ClockIcon fontSize="small" />} label="Time" value={entry.planned_time} /> : null}
          <Chip size="small" variant="outlined" label="Household meal" />
        </FactBar>
      }
      foods={entry.components.map((component) => ({ id: component.id, name: component.item_name, amount: component.amount }))}
      actions={canOptOut ? <Button size="small" disabled={busy} onClick={onOptOut}>Opt out to plan your own</Button> : null}
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
    <Paper variant="outlined" sx={{ px: { xs: 2, sm: 2.5 }, py: 2 }}>
      <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 1 }}>
        <Box sx={{ minWidth: 0 }}>
          <Chip label="Opted out" size="small" />
          <Typography color="text.secondary" sx={{ mt: 0.75 }}>
            Household meal{entry.planned_time ? ` at ${entry.planned_time}` : ''}
          </Typography>
        </Box>
        <Button size="small" disabled={busy} onClick={onJoin}>Join meal</Button>
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
    <Box>
      <PageHeader
        title="My planner"
        actions={canPlan ? <MealSlotMenu choices={headerChoices} onSelect={(slot) => openEditor(null, slot)} /> : null}
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
          meal={editing.entry ? entryToPlannerMeal(editing.entry) : null}
        />
      ) : null}
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
