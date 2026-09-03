import AddIcon from '@mui/icons-material/AddOutlined';
import CheckCircleIcon from '@mui/icons-material/CheckCircleOutlined';
import HelpIcon from '@mui/icons-material/HelpOutlineOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUncheckedOutlined';
import RemoveCircleOutlineIcon from '@mui/icons-material/RemoveCircleOutlineOutlined';
import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { ApiError, type MealItem, type MealPlanEntry, type MealSlot } from '../../api/client';
import { collectStockOutcomes, describeStockOutcome } from './stockShortfall';
import {
  useMealPlanWeek,
  useMarkMealPlanComponentEaten,
  useMarkMealPlanEaten,
  useMeta,
  useReopenMealPlanComponent,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import { AddFoodDialog } from './AddFoodDialog';
import { AteSomethingElseDialog } from './AteSomethingElseDialog';
import { addDays, combineDateTime, defaultDayFor, extractTime, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { EditFoodDialog } from './EditFoodDialog';
import { formatAmount } from './format';
import { DayWeekNutrition } from './NutritionSummary';
import { SLOTS } from './slots';
import { WeekNavigator } from './WeekNavigator';

const DEFAULT_SLOT: MealSlot = SLOTS[0]?.value ?? 'breakfast';

type AddSelection = {
  key: string;
  date: string;
  slot: MealSlot;
};

type EditSelection = {
  key: string;
  date: string;
  slot: MealSlot;
  item: MealItem;
  entry: MealPlanEntry | null;
};

function longDayName(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function displayedEnergy(items: MealItem[]): number | null {
  const values = items
    .filter((item) => item.status !== 'not_eaten')
    .map((item) => item.nutrition.energy_kcal)
    .filter((value): value is number => value != null);
  return values.length > 0 ? values.reduce((total, value) => total + value, 0) : null;
}

function StatusIcon({ status }: { status: MealItem['status'] }) {
  if (status === 'eaten') return <CheckCircleIcon />;
  if (status === 'not_eaten') return <RemoveCircleOutlineIcon />;
  if (status === 'assumed') return <HelpIcon />;
  return <RadioButtonUncheckedIcon />;
}

function statusColour(status: MealItem['status']) {
  if (status === 'eaten') return 'success.main';
  if (status === 'assumed') return 'warning.main';
  return 'text.disabled';
}

function MealItemRow({
  item,
  divided,
  toggling,
  onToggle,
  onOpen,
  unplanned,
}: {
  item: MealItem;
  divided: boolean;
  toggling: boolean;
  onToggle: (() => void) | null;
  onOpen: (() => void) | null;
  unplanned: boolean;
}) {
  const detail = [
    item.consumed_at ? extractTime(item.consumed_at) : item.kind === 'logged' ? item.at : null,
    formatAmount(item.amount),
    item.planned_amount ? `Planned ${formatAmount(item.planned_amount)}` : null,
  ]
    .filter(Boolean)
    .join(' · ');

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        borderTop: divided ? '1px solid' : 'none',
        borderColor: 'divider',
      }}
    >
      {onToggle ? (
        <IconButton
          aria-label={
            item.status === 'not_eaten'
              ? `Reopen ${item.item_name}`
              : item.status === 'eaten'
                ? `Mark ${item.item_name} not eaten yet`
                : `Mark ${item.item_name} eaten`
          }
          onClick={onToggle}
          disabled={toggling}
          sx={{ ml: { xs: 1, sm: 1.5 }, color: statusColour(item.status) }}
        >
          <StatusIcon status={item.status} />
        </IconButton>
      ) : (
        <Box
          sx={{
            ml: { xs: 1, sm: 1.5 },
            p: 1,
            display: 'flex',
            color: statusColour(item.status),
          }}
          aria-hidden
        >
          <StatusIcon status={item.status} />
        </Box>
      )}
      <ButtonBase
        onClick={onOpen ?? undefined}
        disabled={!onOpen}
        aria-label={`Open ${item.item_name}`}
        sx={{
          display: 'flex',
          flexGrow: 1,
          minWidth: 0,
          alignItems: 'center',
          gap: { xs: 1.5, sm: 2 },
          pr: { xs: 2, sm: 2.5 },
          py: 1.75,
          textAlign: 'left',
          opacity: item.status === 'not_eaten' ? 0.55 : 1,
          transition: 'background-color 120ms ease',
          '&:hover': { backgroundColor: 'action.hover' },
          '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main', outlineOffset: -2 },
        }}
      >
        <InitialsAvatar name={item.item_name} size={44} />
        <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
          <Typography
            variant="subtitle1"
            sx={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              textDecoration: item.status === 'not_eaten' ? 'line-through' : 'none',
            }}
          >
            {item.item_name}
          </Typography>
          {item.status === 'assumed' ? (
            <Chip size="small" variant="outlined" color="warning" label="Assumed" sx={{ width: 'fit-content' }} />
          ) : null}
          {unplanned ? (
            <Chip size="small" variant="outlined" color="warning" label="Unplanned" sx={{ width: 'fit-content' }} />
          ) : null}
          {item.item_kind === 'recipe' ? (
            <Chip size="small" variant="outlined" label="Recipe" sx={{ width: 'fit-content' }} />
          ) : null}
          {detail ? (
            <Typography variant="caption" color="text.secondary">
              {detail}
            </Typography>
          ) : null}
        </Stack>
        {item.needs_attention ? (
          <Chip
            size="small"
            variant="outlined"
            icon={<WarningAmberIcon />}
            label="Needs attention"
            sx={{ display: { xs: 'none', sm: 'inline-flex' }, flexShrink: 0 }}
          />
        ) : null}
        {item.status !== 'not_eaten' ? (
          <Box
            sx={{
              display: 'flex',
              flexShrink: 0,
              justifyContent: 'flex-end',
              width: { xs: 'auto', sm: '6rem' },
            }}
          >
            {item.quality === 'unknown' ? (
              <Chip size="small" color="warning" variant="outlined" label="No nutrition" />
            ) : (
              <Typography className="numeral" variant="body2" sx={{ fontWeight: 600, textAlign: 'right' }}>
                {item.quality === 'estimated' ? '~' : null}
                <MaybeNumber value={item.nutrition.energy_kcal} fractionDigits={0} />{' '}
                <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                  kcal
                </Box>
              </Typography>
            )}
          </Box>
        ) : null}
        <ChevronRightIcon sx={{ color: 'text.disabled', fontSize: 20, flexShrink: 0 }} />
      </ButtonBase>
    </Box>
  );
}

function SlotSection({
  slot,
  label,
  items,
  kcal,
  toggling,
  onToggle,
  onOpen,
  onAdd,
  allowChanges,
  onMarkRemaining,
  onAteSomethingElse,
  entries,
}: {
  slot: MealSlot;
  label: string;
  items: MealItem[];
  kcal: number | null;
  toggling: string | null;
  onToggle: (item: MealItem) => void;
  onOpen: (item: MealItem) => void;
  onAdd: (slot: MealSlot) => void;
  allowChanges: boolean;
  onMarkRemaining: (entryId: string, items: MealItem[]) => void;
  onAteSomethingElse: (entryId: string) => void;
  entries: MealPlanEntry[];
}) {
  const slotEntry = entries.find((entry) => entry.slot === slot);
  const groups = new Map<string, MealItem[]>();
  const firstPlannedGroup = new Map<string, string>();
  for (const item of items) {
    const itemKey = item.kind === 'planned' ? item.component_id : item.record_id;
    const groupKey = `item:${itemKey}`;
    const group = groups.get(groupKey) ?? [];
    group.push(item);
    groups.set(groupKey, group);
    if (item.kind === 'planned' && !firstPlannedGroup.has(item.entry_id)) {
      firstPlannedGroup.set(item.entry_id, groupKey);
    }
  }
  return (
    <Box component="section" aria-label={label}>
      <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', minHeight: 30, mb: 1 }}>
        <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
          <Typography variant="overline" color="text.secondary">
            {label}
          </Typography>
          {slot !== 'snacks' && slotEntry?.planned_time ? (
            <Typography variant="overline" color="text.secondary">
              · {slotEntry.planned_time}
            </Typography>
          ) : null}
        </Stack>
        {kcal !== null ? (
          <Typography variant="caption" color="text.secondary">
            {Math.round(kcal)} kcal
          </Typography>
        ) : null}
      </Stack>
      {items.length === 0 ? (
        allowChanges ? (
          <Button
            fullWidth
            startIcon={<AddIcon />}
            onClick={() => onAdd(slot)}
            sx={{
              justifyContent: 'flex-start',
              py: 1.5,
              px: 2,
              color: 'text.secondary',
              border: '1px dashed',
              borderColor: 'divider',
              borderRadius: 2,
            }}
          >
            Add food
          </Button>
        ) : (
          <Paper variant="outlined" sx={{ px: 2, py: 1.5 }}>
            <Typography variant="body2" color="text.secondary">Nothing planned</Typography>
          </Paper>
        )
      ) : (
        <Paper sx={{ overflow: 'hidden' }}>
          {Array.from(groups.entries()).map(([groupKey, group], groupIndex) => {
            const planned = group[0]?.kind === 'planned';
            const entryId = planned && group[0]?.kind === 'planned' ? group[0].entry_id : null;
            // 2026-09-01 - SB: Claude introduced a regression in the planner rework with snacks.
            // by reintroducing the header between planned snacks. This isn't needed for Snacks
            // as they are one long list of snacks differentiated per-row and by the "Unplanned" chip.
            const showPlannedHeader =
              slot !== 'snacks' && entryId !== null && firstPlannedGroup.get(entryId) === groupKey;
            const pending = entryId === null
              ? []
              : items.filter(
                  (item) =>
                    item.kind === 'planned'
                    && item.entry_id === entryId
                    && (item.status === 'planned' || item.status === 'assumed'),
                );
            return (
              <Box
                key={groupKey}
                sx={{ borderTop: groupIndex > 0 ? '1px solid' : 'none', borderColor: 'divider' }}
              >
                {showPlannedHeader ? (
                  <Stack
                    direction="row"
                    spacing={1}
                    sx={{ px: 2, py: 1, alignItems: 'center', justifyContent: 'space-between', backgroundColor: 'action.hover' }}
                  >
                    <Typography variant="caption" color="text.secondary">
                      Planned meal
                    </Typography>
                    {allowChanges && pending.length > 0 && entryId ? (
                      <Stack direction="row" spacing={1}>
                        <Button size="small" onClick={() => onMarkRemaining(entryId, pending)}>
                          Mark remaining eaten
                        </Button>
                        {pending.some((candidate) => candidate.status === 'assumed') ? (
                          <Button size="small" onClick={() => onAteSomethingElse(entryId)}>
                            Ate something else
                          </Button>
                        ) : null}
                      </Stack>
                    ) : null}
                  </Stack>
                ) : null}
                {group.map((item, index) => {
                  const key = item.kind === 'planned' ? item.component_id : item.record_id;
                  return (
                    <MealItemRow
                      key={key}
                      item={item}
                      divided={index > 0 || showPlannedHeader}
                      toggling={toggling === key}
                      onToggle={allowChanges && item.kind === 'planned' ? () => onToggle(item) : null}
                      onOpen={allowChanges ? () => onOpen(item) : null}
                      unplanned={item.kind === 'logged'}
                    />
                  );
                })}
              </Box>
            );
          })}
          {allowChanges ? (
            <Button
              fullWidth
              startIcon={<AddIcon />}
              onClick={() => onAdd(slot)}
              sx={{ py: 1.25, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}
            >
              Add food
            </Button>
          ) : null}
        </Paper>
      )}
    </Box>
  );
}

export function MealPlanPage({ weekStart, day }: { weekStart: string; day: string }) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const week = useMealPlanWeek(weekStart);
  const meta = useMeta();
  const directions = meta.data?.nutrient_directions ?? {};
  const [adding, setAdding] = useState<AddSelection | null>(null);
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const [stockNotice, setStockNotice] = useState<string[]>([]);
  const [toggleError, setToggleError] = useState<string | null>(null);
  const [replacing, setReplacing] = useState<string | null>(null);
  const markEaten = useMarkMealPlanEaten();
  const markComponentEaten = useMarkMealPlanComponentEaten();
  const reopenComponent = useReopenMealPlanComponent();

  const currentMonday = startOfWeekIso(todayIso());
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;

  function goToWeek(start: string) {
    void navigate({
      to: '/food-log/$weekStart/$day',
      params: { weekStart: start, day: defaultDayFor(start) },
    });
  }

  function goToDay(date: string) {
    void navigate({
      to: '/food-log/$weekStart/$day',
      params: { weekStart, day: date },
    });
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  const selectedDay = week.data?.days.find((candidate) => candidate.date === activeDate) ?? week.data?.days[0];

  function addFood(slot: MealSlot) {
    if (!selectedDay) return;
    setAdding({ key: crypto.randomUUID(), date: selectedDay.date, slot });
  }

  function openItem(slot: MealSlot, item: MealItem) {
    if (!selectedDay) return;
    const entry = item.kind === 'planned'
      ? selectedDay.entries.find((candidate) => candidate.id === item.entry_id) ?? null
      : null;
    setEditing({ key: crypto.randomUUID(), date: selectedDay.date, slot, item, entry });
  }

  async function toggleItem(item: MealItem) {
    if (item.kind !== 'planned') return;
    const key = item.component_id;
    setToggling(key);
    try {
      if (item.status === 'planned' || item.status === 'assumed') {
        const updated = await markComponentEaten.mutateAsync({
          id: item.entry_id,
          componentId: item.component_id,
          revision: item.revision,
          body: {
            consumed_on: activeDate,
            consumed_at: item.at ? combineDateTime(activeDate, item.at) : null,
            amount: item.amount,
          },
        });
        setStockNotice(collectStockOutcomes([updated]).map(describeStockOutcome));
      } else {
        await reopenComponent.mutateAsync({
          id: item.entry_id,
          componentId: item.component_id,
          revision: item.revision,
        });
      }
      setToggleError(null);
    } catch (caught) {
      setToggleError(caught instanceof ApiError ? caught.message : 'Could not update this item.');
    } finally {
      setToggling(null);
    }
  }

  async function markRemaining(entryId: string, items: MealItem[]) {
    const entry = selectedDay?.entries.find((candidate) => candidate.id === entryId);
    if (!entry) return;
    setToggling(entryId);
    try {
      const updated = await markEaten.mutateAsync({
        id: entryId,
        revision: entry.revision,
        body: {
          consumed_on: activeDate,
          consumed_at: entry.planned_time ? combineDateTime(activeDate, entry.planned_time) : null,
          components: items.flatMap((item) =>
            item.kind === 'planned' ? [{ component_id: item.component_id, amount: item.amount }] : [],
          ),
        },
      });
      setStockNotice(collectStockOutcomes([updated]).map(describeStockOutcome));
      setToggleError(null);
    } catch (caught) {
      setToggleError(caught instanceof ApiError ? caught.message : 'Could not update this meal.');
    } finally {
      setToggling(null);
    }
  }

  const future = activeDate > todayIso();
  const allowChanges = !future;

  return (
    <Box>
      <PageHeader
        title="Food log"
        actions={
          selectedDay && allowChanges ? (
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => addFood(DEFAULT_SLOT)}>
              Add food
            </Button>
          ) : null
        }
      />

      {toggleError ? (
        <Alert severity="error" onClose={() => setToggleError(null)} sx={{ mb: 3 }}>
          {toggleError}
        </Alert>
      ) : null}

      {stockNotice.length > 0 ? (
        <Alert severity="warning" onClose={() => setStockNotice([])} sx={{ mb: 3 }}>
          {stockNotice.map((line) => (
            <div key={line}>{line}</div>
          ))}
        </Alert>
      ) : null}

      {week.data ? (
        <WeekNavigator
          weekStart={weekStart}
          days={week.data.days.map((candidate) => ({
            date: candidate.date,
            itemCount: candidate.slots.reduce((sum, slot) => sum + slot.items.length, 0),
          }))}
          selectedDate={activeDate}
          currentMonday={currentMonday}
          onWeekChange={goToWeek}
          onDayChange={goToDay}
        />
      ) : null}

      {week.isLoading ? <Loading label="Loading week" /> : null}
      {week.data && selectedDay ? (
        <Box component="section" aria-labelledby="day-plan-heading">
          <Typography id="day-plan-heading" variant="h2" sx={{ mb: 1.25 }}>
            {longDayName(selectedDay.date)}
          </Typography>

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

            {selectedDay.slots.map((slotView) => (
              <SlotSection
                key={slotView.slot}
                slot={slotView.slot}
                label={SLOTS.find((candidate) => candidate.value === slotView.slot)?.label ?? slotView.slot}
                items={slotView.items}
                kcal={displayedEnergy(slotView.items)}
                toggling={toggling}
                onToggle={toggleItem}
                onOpen={(item) => openItem(slotView.slot, item)}
                onAdd={addFood}
                allowChanges={allowChanges}
                onMarkRemaining={markRemaining}
                onAteSomethingElse={setReplacing}
                entries={selectedDay.entries}
              />
            ))}
          </Stack>
        </Box>
      ) : null}

      {adding ? (
        <AddFoodDialog
          key={adding.key}
          open
          onClose={() => setAdding(null)}
          memberId={memberId}
          date={adding.date}
          slot={adding.slot}
        />
      ) : null}

      {replacing && selectedDay ? (
        <AteSomethingElseDialog
          open
          onClose={() => setReplacing(null)}
          entryId={replacing}
          revision={selectedDay.entries.find((entry) => entry.id === replacing)?.revision ?? 0}
          memberId={memberId}
          consumedOn={activeDate}
        />
      ) : null}

      {editing ? (
        <EditFoodDialog
          key={editing.key}
          open
          onClose={() => setEditing(null)}
          memberId={memberId}
          date={editing.date}
          slot={editing.slot}
          item={editing.item}
          entry={editing.entry}
        />
      ) : null}
    </Box>
  );
}
