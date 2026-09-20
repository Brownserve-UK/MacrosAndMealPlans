import AddIcon from '@mui/icons-material/AddOutlined';
import CheckCircleIcon from '@mui/icons-material/CheckCircleOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import HelpIcon from '@mui/icons-material/HelpOutlineOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUncheckedOutlined';
import RemoveCircleOutlineIcon from '@mui/icons-material/RemoveCircleOutlineOutlined';
import StorefrontIcon from '@mui/icons-material/StorefrontOutlined';
import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import IconButton from '@mui/material/IconButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState, type ReactNode } from 'react';
import { ApiError, type MealItem, type MealPlanEntry, type MealSlot } from '../../api/client';
import { collectStockOutcomes, describeStockOutcome } from './stockShortfall';
import {
  useAddGroup,
  useMealPlanWeek,
  useMarkMealPlanComponentEaten,
  useMeta,
  usePlannerWeek,
  useReopenMealPlanComponent,
  useSetAttendance,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { IconTile } from '../../components/IconTile';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { AddFoodDialog } from './AddFoodDialog';
import { AteSomethingElseDialog } from './AteSomethingElseDialog';
import { addDays, combineDateTime, defaultDayFor, extractTime, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { EditFoodDialog } from './EditFoodDialog';
import { formatAmount } from './format';
import { DayWeekNutrition } from './NutritionSummary';
import { groupDiners, occasionAt, occasionTitle } from './planner/plannerWeek';
import type { GroupView, OccasionView, PlannerWeek } from './planner/types';
import { leftoversRow, leftoverCaption, useLeftoverDishes } from './planner/usePickerRows';
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

export type MyAttendance = {
  occasion: OccasionView;
  group: GroupView | null;
  absent: boolean;
  note: string | null;
};

type NotEating = {
  anchor: HTMLElement;
  occasion: OccasionView;
  group: GroupView;
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

export function myAttendance(
  week: PlannerWeek | undefined,
  date: string,
  slot: MealSlot,
  memberId: string,
): MyAttendance | null {
  const occasion = occasionAt(week, date, slot);
  if (!occasion || !week) return null;
  const group =
    occasion.groups.find((candidate) =>
      groupDiners(occasion, candidate, week.members).some((diner) => diner.id === memberId),
    ) ?? null;
  return {
    occasion,
    group,
    absent: occasion.absent_member_ids.includes(memberId),
    note: group?.participants.find((participant) => participant.member_id === memberId)?.note ?? null,
  };
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
  note,
  from,
}: {
  item: MealItem;
  divided: boolean;
  toggling: boolean;
  onToggle: (() => void) | null;
  onOpen: (() => void) | null;
  unplanned: boolean;
  note: string | null;
  from: string | null;
}) {
  const detail = [
    note,
    item.consumed_at ? extractTime(item.consumed_at) : item.kind === 'logged' ? item.at : null,
    formatAmount(item.amount),
    item.planned_amount ? `Planned ${formatAmount(item.planned_amount)}` : null,
    from ? `from ${from}` : null,
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
          <Stack direction="row" spacing={0.75} sx={{ flexWrap: 'wrap' }}>
            {item.kind === 'planned' && item.status === 'planned' ? (
              <Chip size="small" variant="outlined" label="Planned" />
            ) : null}
            {item.status === 'assumed' ? (
              <Chip size="small" variant="outlined" color="warning" label="Assumed" />
            ) : null}
            {unplanned ? <Chip size="small" variant="outlined" color="warning" label="Unplanned" /> : null}
            {item.item_kind === 'recipe' ? <Chip size="small" variant="outlined" label="Recipe" /> : null}
          </Stack>
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

function OutIcon() {
  return (
    <Box
      aria-hidden
      sx={{
        width: 34,
        height: 34,
        flexShrink: 0,
        borderRadius: '10px',
        display: 'grid',
        placeItems: 'center',
        backgroundColor: 'background.default',
        color: 'text.secondary',
      }}
    >
      <StorefrontIcon sx={{ fontSize: 17 }} />
    </Box>
  );
}

function ElsewhereRow({
  occasion,
  busy,
  onChange,
}: {
  occasion: OccasionView;
  busy: boolean;
  onChange: () => void;
}) {
  return (
    <Stack direction="row" spacing={2} sx={{ alignItems: 'center', px: 2, py: 1.5, opacity: 0.7 }}>
      <OutIcon />
      <Stack sx={{ minWidth: 0, flexGrow: 1 }}>
        <Typography variant="subtitle1">Eating elsewhere</Typography>
        <Typography variant="caption" color="text.secondary">
          {`${occasionTitle(occasion)} · you're out on the planner`}
        </Typography>
      </Stack>
      <Button size="small" color="inherit" onClick={onChange} disabled={busy}>
        Change
      </Button>
    </Stack>
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
  attendance,
  busy,
  onNotEating,
  onChangeAttendance,
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
  attendance: MyAttendance | null;
  busy: boolean;
  onNotEating: (anchor: HTMLElement, occasion: OccasionView, group: GroupView) => void;
  onChangeAttendance: (occasion: OccasionView) => void;
  entries: MealPlanEntry[];
}) {
  const slotEntry = entries.find((entry) => entry.slot === slot);
  const time = attendance?.occasion.effective_time ?? slotEntry?.planned_time ?? null;
  const groups = new Map<string, MealItem[]>();
  const lastPlannedGroup = new Map<string, string>();
  for (const item of items) {
    const itemKey = item.kind === 'planned' ? item.component_id : item.record_id;
    const groupKey = `item:${itemKey}`;
    const group = groups.get(groupKey) ?? [];
    group.push(item);
    groups.set(groupKey, group);
    if (item.kind === 'planned') lastPlannedGroup.set(item.entry_id, groupKey);
  }
  const absent = attendance?.absent ?? false;
  const from = attendance ? occasionTitle(attendance.occasion) : null;
  const addFood = (bordered: boolean) => (
    <Button
      fullWidth
      startIcon={<AddIcon />}
      onClick={() => onAdd(slot)}
      sx={
        bordered
          ? { py: 1.25, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }
          : {
              justifyContent: 'flex-start',
              py: 1.5,
              px: 2,
              color: 'text.secondary',
              border: '1px dashed',
              borderColor: 'divider',
              borderRadius: 2,
            }
      }
    >
      Add food
    </Button>
  );

  return (
    <Box component="section" aria-label={label}>
      <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', minHeight: 30, mb: 1 }}>
        <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
          <Typography variant="overline" color="text.secondary">
            {label}
          </Typography>
          {slot !== 'snacks' && time ? (
            <Typography variant="overline" color="text.secondary">
              · {time}
            </Typography>
          ) : null}
        </Stack>
        {kcal !== null ? (
          <Typography variant="caption" color="text.secondary">
            {Math.round(kcal)} kcal
          </Typography>
        ) : null}
      </Stack>
      {items.length === 0 && !absent ? (
        allowChanges ? (
          addFood(false)
        ) : (
          <Paper variant="outlined" sx={{ px: 2, py: 1.5 }}>
            <Typography variant="body2" color="text.secondary">Nothing planned</Typography>
          </Paper>
        )
      ) : (
        <Paper sx={{ overflow: 'hidden' }}>
          {absent && attendance ? (
            <ElsewhereRow
              occasion={attendance.occasion}
              busy={busy}
              onChange={() => onChangeAttendance(attendance.occasion)}
            />
          ) : null}
          {Array.from(groups.entries()).map(([groupKey, group], groupIndex) => {
            const first = group[0];
            const entryId = first?.kind === 'planned' ? first.entry_id : null;
            const closesEntry = entryId !== null && lastPlannedGroup.get(entryId) === groupKey;
            const mine = entryId !== null && attendance?.group?.id === entryId ? attendance : null;
            const unresolved = group.some((item) => item.status === 'planned' || item.status === 'assumed');
            return (
              <Box
                key={groupKey}
                sx={{ borderTop: groupIndex > 0 || absent ? '1px solid' : 'none', borderColor: 'divider' }}
              >
                {group.map((item, index) => {
                  const key = item.kind === 'planned' ? item.component_id : item.record_id;
                  return (
                    <MealItemRow
                      key={key}
                      item={item}
                      divided={index > 0}
                      toggling={toggling === key}
                      onToggle={allowChanges && item.kind === 'planned' ? () => onToggle(item) : null}
                      onOpen={allowChanges ? () => onOpen(item) : null}
                      unplanned={item.kind === 'logged'}
                      note={item.kind === 'planned' && mine ? mine.note : null}
                      from={item.kind === 'planned' ? from : null}
                    />
                  );
                })}
                {closesEntry && mine?.group && unresolved ? (
                  <Box sx={{ display: 'flex', justifyContent: 'flex-end', px: 1.5, pb: 1 }}>
                    <Button
                      size="small"
                      color="inherit"
                      disabled={busy}
                      sx={{ color: 'text.secondary' }}
                      onClick={(event) => onNotEating(event.currentTarget, mine.occasion, mine.group as GroupView)}
                    >
                      Not eating this
                    </Button>
                  </Box>
                ) : null}
              </Box>
            );
          })}
          {allowChanges ? addFood(true) : null}
        </Paper>
      )}
    </Box>
  );
}

function MenuRow({ icon, primary, secondary }: { icon: ReactNode; primary: string; secondary: string | null }) {
  return (
    <>
      <ListItemIcon sx={{ minWidth: 46 }}>{icon}</ListItemIcon>
      <ListItemText primary={primary} secondary={secondary} />
    </>
  );
}

export function MealPlanPage({ weekStart, day }: { weekStart: string; day: string }) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const week = useMealPlanWeek(weekStart);
  const plannerWeek = usePlannerWeek(weekStart);
  const meta = useMeta();
  const directions = meta.data?.nutrient_directions ?? {};
  const [adding, setAdding] = useState<AddSelection | null>(null);
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const [stockNotice, setStockNotice] = useState<string[]>([]);
  const [toggleError, setToggleError] = useState<string | null>(null);
  const [replacing, setReplacing] = useState<string | null>(null);
  const [notEating, setNotEating] = useState<NotEating | null>(null);
  const markComponentEaten = useMarkMealPlanComponentEaten();
  const reopenComponent = useReopenMealPlanComponent();
  const setAttendance = useSetAttendance();
  const addGroup = useAddGroup();

  const timeZone = useHouseholdTimeZone();
  const currentMonday = startOfWeekIso(todayIso(timeZone));
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;
  const dishes = useLeftoverDishes(notEating?.occasion.planned_on ?? activeDate);

  function goToWeek(start: string) {
    void navigate({
      to: '/my-food/$weekStart/$day',
      params: { weekStart: start, day: defaultDayFor(start, timeZone) },
    });
  }

  function goToDay(date: string) {
    void navigate({
      to: '/my-food/$weekStart/$day',
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

  async function attend(action: () => Promise<unknown>, fallback: string) {
    setNotEating(null);
    try {
      await action();
      setToggleError(null);
    } catch (caught) {
      setToggleError(caught instanceof ApiError ? caught.message : fallback);
    }
  }

  function eatElsewhere(occasion: OccasionView) {
    void attend(
      () => setAttendance.mutateAsync({ occasionId: occasion.id, memberId, attendance: { kind: 'elsewhere' } }),
      'Could not mark you as out.',
    );
  }

  function eatLeftovers(occasion: OccasionView) {
    const dish = dishes[0];
    if (!dish) return;
    void attend(
      () =>
        addGroup.mutateAsync({
          occasionId: occasion.id,
          body: { ...leftoversRow(dish, 1).group, everyone: false, participants: [{ member_id: memberId }] },
        }),
      'Could not switch you to leftovers.',
    );
  }

  function backIn(occasion: OccasionView) {
    const group = occasion.groups.find((candidate) => candidate.everyone) ?? occasion.groups[0] ?? null;
    void attend(
      () =>
        setAttendance.mutateAsync({
          occasionId: occasion.id,
          memberId,
          attendance: group ? { kind: 'eating', group_id: group.id } : { kind: 'unaccounted' },
        }),
      'Could not put you back on the plan.',
    );
  }

  const future = activeDate > todayIso(timeZone);
  const allowChanges = !future;
  const busy = setAttendance.isPending || addGroup.isPending;
  const fridge = dishes[0] ?? null;

  return (
    <Box>
      <PageHeader
        title="My food"
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
                attendance={myAttendance(plannerWeek.data, selectedDay.date, slotView.slot, memberId)}
                busy={busy}
                onNotEating={(anchor, occasion, group) => setNotEating({ anchor, occasion, group })}
                onChangeAttendance={backIn}
                entries={selectedDay.entries}
              />
            ))}
          </Stack>
        </Box>
      ) : null}

      <Menu
        open={Boolean(notEating)}
        anchorEl={notEating?.anchor ?? null}
        onClose={() => setNotEating(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        transformOrigin={{ vertical: 'top', horizontal: 'right' }}
      >
        <MenuItem onClick={() => notEating && eatElsewhere(notEating.occasion)}>
          <MenuRow icon={<OutIcon />} primary="Eating elsewhere" secondary="The planner shows you as out" />
        </MenuItem>
        <MenuItem disabled={!fridge} onClick={() => notEating && eatLeftovers(notEating.occasion)}>
          <MenuRow
            icon={<IconTile concept="dish" tone="secondary" />}
            primary="Leftovers"
            secondary={fridge ? `${fridge.name}, ${leftoverCaption(fridge)}` : 'No leftovers available'}
          />
        </MenuItem>
        <MenuItem
          onClick={() => {
            const group = notEating?.group ?? null;
            setNotEating(null);
            if (group) setReplacing(group.id);
          }}
        >
          <MenuRow
            icon={
              <Box
                aria-hidden
                sx={{
                  width: 34,
                  height: 34,
                  borderRadius: '10px',
                  display: 'grid',
                  placeItems: 'center',
                  backgroundColor: 'background.default',
                  color: 'text.secondary',
                }}
              >
                <EditIcon sx={{ fontSize: 17 }} />
              </Box>
            }
            primary="Something else"
            secondary="Log what you had"
          />
        </MenuItem>
      </Menu>

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
          revision={
            plannerWeek.data?.days
              .flatMap((candidate) => candidate.occasions)
              .flatMap((occasion) => occasion?.groups ?? [])
              .find((group) => group.id === replacing)?.revision ??
            selectedDay.entries.find((entry) => entry.id === replacing)?.revision ??
            0
          }
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
