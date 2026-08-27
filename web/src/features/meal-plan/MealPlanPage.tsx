import AccessTimeIcon from '@mui/icons-material/AccessTimeOutlined';
import AddIcon from '@mui/icons-material/AddOutlined';
import CheckCircleIcon from '@mui/icons-material/CheckCircleOutlined';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUncheckedOutlined';
import RemoveCircleOutlineIcon from '@mui/icons-material/RemoveCircleOutlineOutlined';
import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Popover from '@mui/material/Popover';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import type { MealItem, MealPlanDay, MealPlanEntry, MealSlot } from '../../api/client';
import {
  useMealPlanWeek,
  useMarkMealPlanComponentEaten,
  useMarkMealPlanEaten,
  useMeta,
  useReopenMealPlanComponent,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import { AddFoodDialog } from './AddFoodDialog';
import { addDays, combineDateTime, extractTime, formatWeekRange, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { EditFoodDialog } from './EditFoodDialog';
import { formatAmount } from './format';
import { DayWeekNutrition } from './NutritionSummary';
import { SLOTS } from './slots';

const DEFAULT_SLOT: MealSlot = SLOTS[0]?.value ?? 'breakfast';
type MealWorkspace = 'today' | 'planner';

type AddSelection = {
  key: string;
  date: string;
  slot: MealSlot;
  entry: MealPlanEntry | null;
};

type EditSelection = {
  key: string;
  date: string;
  slot: MealSlot;
  item: MealItem;
  entry: MealPlanEntry | null;
};

export function defaultDayFor(weekStart: string): string {
  const today = todayIso();
  return today >= weekStart && today <= addDays(weekStart, 6) ? today : weekStart;
}

function longDayName(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function displayedEnergy(items: MealItem[], workspace: MealWorkspace): number | null {
  const values = items
    .filter((item) => workspace === 'today' || item.kind === 'planned')
    .filter((item) => item.status !== 'not_eaten')
    .map((item) => item.nutrition.energy_kcal)
    .filter((value): value is number => value != null);
  return values.length > 0 ? values.reduce((total, value) => total + value, 0) : null;
}

function MealItemRow({
  item,
  divided,
  toggling,
  onToggle,
  onOpen,
  unplanned,
  passiveStatus,
}: {
  item: MealItem;
  divided: boolean;
  toggling: boolean;
  onToggle: (() => void) | null;
  onOpen: (() => void) | null;
  unplanned: boolean;
  passiveStatus: boolean;
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
              ? `Reopen ${item.product_name}`
              : item.status === 'planned'
                ? `Mark ${item.product_name} eaten`
                : `Mark ${item.product_name} not eaten yet`
          }
          onClick={onToggle}
          disabled={toggling}
          sx={{
            ml: { xs: 1, sm: 1.5 },
            color: item.status === 'eaten' ? 'success.main' : 'text.disabled',
          }}
        >
          {item.status === 'eaten' ? (
            <CheckCircleIcon />
          ) : item.status === 'not_eaten' ? (
            <RemoveCircleOutlineIcon />
          ) : (
            <RadioButtonUncheckedIcon />
          )}
        </IconButton>
      ) : passiveStatus ? null : (
        <Box
          sx={{
            ml: { xs: 1, sm: 1.5 },
            p: 1,
            display: 'flex',
            color: item.status === 'eaten' ? 'success.main' : 'text.disabled',
          }}
          aria-hidden
        >
          {item.status === 'eaten' ? (
            <CheckCircleIcon />
          ) : item.status === 'not_eaten' ? (
            <RemoveCircleOutlineIcon />
          ) : (
            <RadioButtonUncheckedIcon />
          )}
        </Box>
      )}
      <ButtonBase
        onClick={onOpen ?? undefined}
        disabled={!onOpen}
        aria-label={`Open ${item.product_name}`}
        sx={{
          display: 'flex',
          flexGrow: 1,
          minWidth: 0,
          alignItems: 'center',
          gap: { xs: 1.5, sm: 2 },
          pl: passiveStatus ? { xs: 2, sm: 2.5 } : 0,
          pr: { xs: 2, sm: 2.5 },
          py: 1.75,
          textAlign: 'left',
          opacity: item.status === 'not_eaten' ? 0.55 : 1,
          transition: 'background-color 120ms ease',
          '&:hover': { backgroundColor: 'action.hover' },
          '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main', outlineOffset: -2 },
        }}
      >
        <InitialsAvatar name={item.product_name} size={44} />
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
            {item.product_name}
          </Typography>
          {unplanned ? (
            <Chip size="small" variant="outlined" color="warning" label="Unplanned" sx={{ width: 'fit-content' }} />
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
        {passiveStatus ? (
          <Box sx={{ display: 'flex', flexShrink: 0, width: { xs: 'auto', sm: '6rem' } }}>
            <Chip
              size="small"
              variant="outlined"
              color={item.status === 'eaten' ? 'success' : item.status === 'not_eaten' ? 'default' : 'info'}
              label={item.status === 'eaten' ? 'Eaten' : item.status === 'not_eaten' ? 'Not eaten' : 'Planned'}
            />
          </Box>
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
              <Chip size="small" variant="outlined" label="No nutrition" />
            ) : (
              <Typography className="numeral" variant="body2" sx={{ fontWeight: 600, textAlign: 'right' }}>
                <MaybeNumber value={item.nutrition.energy_kcal} fractionDigits={0} />{' '}
                <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                  kcal
                </Box>
              </Typography>
            )}
          </Box>
        ) : passiveStatus ? (
          <Box aria-hidden sx={{ flexShrink: 0, width: { xs: 0, sm: '6rem' } }} />
        ) : null}
        <ChevronRightIcon sx={{ color: 'text.disabled', fontSize: 20, flexShrink: 0 }} />
      </ButtonBase>
    </Box>
  );
}

function SlotTimeControl({ entry }: { entry: MealPlanEntry }) {
  const update = useUpdateMealPlanEntry();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [value, setValue] = useState(entry.planned_time ?? '');
  const editable = entry.status === 'planned';

  if (!editable) {
    return entry.planned_time ? (
      <Typography variant="overline" color="text.secondary">
        · {entry.planned_time}
      </Typography>
    ) : null;
  }

  function open(event: React.MouseEvent<HTMLElement>) {
    setValue(entry.planned_time ?? '');
    setAnchor(event.currentTarget);
  }

  async function save(next: string | null) {
    const ok = await update
      .mutateAsync({ id: entry.id, revision: entry.revision, body: { planned_time: next } })
      .then(() => true, () => false);
    if (ok) setAnchor(null);
  }

  return (
    <>
      <Button
        size="small"
        onClick={open}
        startIcon={<AccessTimeIcon sx={{ fontSize: 15 }} />}
        sx={{
          minWidth: 0,
          py: 0,
          px: 0.5,
          color: 'text.secondary',
          textTransform: 'none',
          fontWeight: 400,
        }}
      >
        {entry.planned_time ?? 'Add time'}
      </Button>
      <Popover
        open={Boolean(anchor)}
        anchorEl={anchor}
        onClose={() => setAnchor(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      >
        <Stack spacing={1.5} sx={{ p: 2, width: 220 }}>
          <TextField
            type="time"
            label="Planned meal time"
            value={value}
            onChange={(event) => setValue(event.target.value)}
            slotProps={{ inputLabel: { shrink: true } }}
            autoFocus
            fullWidth
          />
          <Stack direction="row" spacing={1} sx={{ justifyContent: 'space-between' }}>
            <Button
              size="small"
              color="inherit"
              disabled={!entry.planned_time || update.isPending}
              onClick={() => void save(null)}
            >
              Clear
            </Button>
            <Button
              size="small"
              variant="contained"
              disabled={!value || update.isPending}
              onClick={() => void save(value)}
            >
              Save
            </Button>
          </Stack>
        </Stack>
      </Popover>
    </>
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
  workspace = 'today',
  onMarkRemaining,
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
  workspace?: MealWorkspace;
  onMarkRemaining: (entryId: string, items: MealItem[]) => void;
  entries: MealPlanEntry[];
}) {
  const visibleItems = workspace === 'planner' ? items.filter((item) => item.kind === 'planned') : items;
  const slotEntry = entries.find((entry) => entry.slot === slot);
  const groups = new Map<string, MealItem[]>();
  const firstPlannedGroup = new Map<string, string>();
  for (const item of visibleItems) {
    const itemKey = item.kind === 'planned' ? item.component_id : item.record_id;
    const groupKey = workspace === 'planner' && item.kind === 'planned' ? item.entry_id : `item:${itemKey}`;
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
          {workspace === 'planner' && slotEntry ? (
            <SlotTimeControl entry={slotEntry} />
          ) : slotEntry?.planned_time ? (
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
      {visibleItems.length === 0 ? (
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
            {workspace === 'planner' ? 'Add planned food' : 'Add food'}
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
            const showPlannedHeader = entryId !== null &&
              (workspace === 'planner' || firstPlannedGroup.get(entryId) === groupKey);
            const pending = entryId === null
              ? []
              : visibleItems.filter(
                  (item) => item.kind === 'planned' && item.entry_id === entryId && item.status === 'planned',
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
                    {workspace === 'today' && allowChanges && pending.length > 0 && entryId ? (
                      <Button size="small" onClick={() => onMarkRemaining(entryId, pending)}>
                        Mark remaining eaten
                      </Button>
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
                      onToggle={allowChanges && workspace === 'today' && item.kind === 'planned' ? () => onToggle(item) : null}
                      onOpen={
                        allowChanges &&
                        (item.kind === 'logged' ||
                          (workspace === 'planner' && item.status === 'planned') ||
                          (workspace === 'today' && item.kind === 'planned'))
                          ? () => onOpen(item)
                          : null
                      }
                      unplanned={item.kind === 'logged'}
                      passiveStatus={workspace === 'planner'}
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
              {workspace === 'planner' ? 'Add planned food' : 'Add food'}
            </Button>
          ) : null}
        </Paper>
      )}
    </Box>
  );
}

function WeekDayRail({
  days,
  value,
  onChange,
}: {
  days: MealPlanDay[];
  value: string;
  onChange: (date: string) => void;
}) {
  return (
    <Box sx={{ display: 'flex', overflowX: 'auto', p: 0.75 }}>
      {days.map((day) => {
        const selected = day.date === value;
        const date = parseIsoDate(day.date);
        const itemCount = day.slots.reduce((sum, slot) => sum + slot.items.length, 0);
        return (
          <ButtonBase
            key={day.date}
            onClick={() => onChange(day.date)}
            aria-pressed={selected}
            aria-label={`${longDayName(day.date)}, ${itemCount} ${itemCount === 1 ? 'item' : 'items'}`}
            sx={{
              minWidth: { xs: 68, sm: 0 },
              flex: { sm: 1 },
              px: 1,
              py: 1,
              borderRadius: 1.5,
              backgroundColor: selected ? 'action.selected' : 'transparent',
              '&:hover': { backgroundColor: 'action.hover' },
              '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main' },
            }}
          >
            <Stack spacing={0.25} sx={{ alignItems: 'center' }}>
              <Typography variant="caption" color={selected ? 'text.primary' : 'text.secondary'}>
                {date.toLocaleDateString('en-GB', { weekday: 'short' })}
              </Typography>
              <Typography className="numeral" sx={{ fontWeight: 650 }}>
                {date.getDate()}
              </Typography>
              <Box
                aria-hidden
                sx={{
                  width: 5,
                  height: 5,
                  borderRadius: '50%',
                  backgroundColor: itemCount > 0 ? 'primary.main' : 'transparent',
                }}
              />
            </Stack>
          </ButtonBase>
        );
      })}
    </Box>
  );
}

function WeekNavigator({
  weekStart,
  days,
  selectedDate,
  currentMonday,
  onWeekChange,
  onDayChange,
}: {
  weekStart: string;
  days: MealPlanDay[];
  selectedDate: string;
  currentMonday: string;
  onWeekChange: (week: string) => void;
  onDayChange: (date: string) => void;
}) {
  return (
    <Paper sx={{ mb: 3, overflow: 'hidden', backgroundColor: 'background.default' }}>
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', sm: '1fr auto 1fr' },
          alignItems: 'center',
          gap: 2,
          px: { xs: 1.5, sm: 2 },
          py: 1.5,
        }}
      >
        <Box sx={{ display: { xs: 'none', sm: 'block' } }}>
          {weekStart !== currentMonday ? (
            <Button size="small" onClick={() => onWeekChange(currentMonday)}>
              This week
            </Button>
          ) : null}
        </Box>
        <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', justifyContent: 'center' }}>
          <IconButton
            size="small"
            aria-label="Previous week"
            onClick={() => onWeekChange(addDays(weekStart, -7))}
            sx={{ border: '1px solid', borderColor: 'divider' }}
          >
            <ChevronLeftIcon />
          </IconButton>
          <Typography variant="h3" sx={{ minWidth: { xs: 168, sm: 208 }, textAlign: 'center' }}>
            {formatWeekRange(weekStart)}
          </Typography>
          <IconButton
            size="small"
            aria-label="Next week"
            onClick={() => onWeekChange(addDays(weekStart, 7))}
            sx={{ border: '1px solid', borderColor: 'divider' }}
          >
            <ChevronRightIcon />
          </IconButton>
        </Stack>
        <Box sx={{ display: { xs: 'none', sm: 'block' } }} />
      </Box>
      <Divider />
      <WeekDayRail days={days} value={selectedDate} onChange={onDayChange} />
    </Paper>
  );
}

export function MealPlanPage({
  weekStart,
  day,
  workspace = 'today',
}: {
  weekStart: string;
  day: string;
  workspace?: MealWorkspace;
}) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const week = useMealPlanWeek(weekStart);
  const meta = useMeta();
  const directions = meta.data?.nutrient_directions ?? {};
  const [adding, setAdding] = useState<AddSelection | null>(null);
  const [editing, setEditing] = useState<EditSelection | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const markEaten = useMarkMealPlanEaten();
  const markComponentEaten = useMarkMealPlanComponentEaten();
  const reopenComponent = useReopenMealPlanComponent();

  const currentMonday = startOfWeekIso(todayIso());
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;

  function goToWeek(start: string) {
    void navigate({
      to: workspace === 'today' ? '/food-log/$weekStart/$day' : '/planner/$weekStart/$day',
      params: { weekStart: start, day: defaultDayFor(start) },
    });
  }

  function goToDay(date: string) {
    void navigate({
      to: workspace === 'today' ? '/food-log/$weekStart/$day' : '/planner/$weekStart/$day',
      params: { weekStart, day: date },
    });
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  const selectedDay = week.data?.days.find((candidate) => candidate.date === activeDate) ?? week.data?.days[0];

  function addFood(slot: MealSlot) {
    if (!selectedDay) return;
    const entry = workspace === 'planner'
      ? selectedDay.entries.find(
          (candidate) => candidate.slot === slot && candidate.status === 'planned',
        ) ?? null
      : null;
    setAdding({ key: crypto.randomUUID(), date: selectedDay.date, slot, entry });
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
      if (item.status === 'planned') {
        await markComponentEaten.mutateAsync({
          id: item.entry_id,
          componentId: item.component_id,
          revision: item.revision,
          body: {
            consumed_on: activeDate,
            consumed_at: item.at ? combineDateTime(activeDate, item.at) : null,
            amount: item.amount,
          },
        });
      } else {
        await reopenComponent.mutateAsync({
          id: item.entry_id,
          componentId: item.component_id,
          revision: item.revision,
        });
      }
    } finally {
      setToggling(null);
    }
  }

  async function markRemaining(entryId: string, items: MealItem[]) {
    const entry = selectedDay?.entries.find((candidate) => candidate.id === entryId);
    if (!entry) return;
    setToggling(entryId);
    try {
      await markEaten.mutateAsync({
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
    } finally {
      setToggling(null);
    }
  }

  const future = activeDate > todayIso();
  const allowChanges = workspace === 'planner' || !future;

  return (
    <Box>
      <PageHeader
        title={workspace === 'today' ? 'Food log' : 'Planner'}
        actions={
          selectedDay && allowChanges ? (
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => addFood(DEFAULT_SLOT)}>
              {workspace === 'planner' ? 'Add planned food' : 'Add food'}
            </Button>
          ) : null
        }
      />

      {workspace === 'today' && future ? (
        <Paper variant="outlined" sx={{ p: 2, mb: 3 }}>
          <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}>
            <Typography variant="body2">Future food is read-only in the food log.</Typography>
            <Button onClick={() => void navigate({ to: '/planner/$weekStart/$day', params: { weekStart, day: activeDate } })}>
              Open in Planner
            </Button>
          </Stack>
        </Paper>
      ) : null}

      {week.data ? (
        <WeekNavigator
          weekStart={weekStart}
          days={week.data.days}
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
                kcal={displayedEnergy(slotView.items, workspace)}
                toggling={toggling}
                onToggle={toggleItem}
                onOpen={(item) => openItem(slotView.slot, item)}
                onAdd={addFood}
                allowChanges={allowChanges}
                workspace={workspace}
                onMarkRemaining={markRemaining}
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
          kind={workspace === 'planner' ? 'planned' : 'eaten'}
          entry={adding.entry}
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
          workspace={workspace}
        />
      ) : null}
    </Box>
  );
}
