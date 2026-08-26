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
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import type { MealItem, MealPlanDay, MealSlot } from '../../api/client';
import { useMealPlanWeek, useMeta, useMarkMealPlanEaten, useReopenMealPlanEntry } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import { AddFoodDialog } from './AddFoodDialog';
import { addDays, formatWeekRange, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { EditFoodDialog } from './EditFoodDialog';
import { formatAmount } from './format';
import { DayWeekNutrition } from './NutritionSummary';
import { SLOTS } from './slots';

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

function MealItemRow({
  item,
  divided,
  toggling,
  onToggle,
  onOpen,
}: {
  item: MealItem;
  divided: boolean;
  toggling: boolean;
  onToggle: (() => void) | null;
  onOpen: () => void;
}) {
  const detail = [
    item.at,
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
      ) : (
        <Box sx={{ ml: { xs: 1, sm: 1.5 }, p: 1, display: 'flex', color: 'success.main' }} aria-hidden>
          <CheckCircleIcon />
        </Box>
      )}
      <ButtonBase
        onClick={onOpen}
        aria-label={`Open ${item.product_name}`}
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
          item.quality === 'unknown' ? (
            <Chip size="small" variant="outlined" label="No nutrition" sx={{ flexShrink: 0 }} />
          ) : (
            <Typography className="numeral" variant="body2" sx={{ flexShrink: 0, fontWeight: 600 }}>
              <MaybeNumber value={item.nutrition.energy_kcal} fractionDigits={0} />{' '}
              <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                kcal
              </Box>
            </Typography>
          )
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
}: {
  slot: MealSlot;
  label: string;
  items: MealItem[];
  kcal: number | null;
  toggling: string | null;
  onToggle: (item: MealItem) => void;
  onOpen: (item: MealItem) => void;
  onAdd: (slot: MealSlot) => void;
}) {
  return (
    <Box component="section" aria-label={label}>
      <Stack direction="row" sx={{ alignItems: 'baseline', justifyContent: 'space-between', mb: 1 }}>
        <Typography variant="overline" color="text.secondary">
          {label}
        </Typography>
        {kcal !== null ? (
          <Typography variant="caption" color="text.secondary">
            {Math.round(kcal)} kcal
          </Typography>
        ) : null}
      </Stack>
      {items.length === 0 ? (
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
        <Paper sx={{ overflow: 'hidden' }}>
          {items.map((item, index) => {
            const key =
              item.kind === 'planned' ? item.component_id : item.record_id;
            return (
              <MealItemRow
                key={key}
                item={item}
                divided={index > 0}
                toggling={toggling === key}
                onToggle={item.kind === 'planned' ? () => onToggle(item) : null}
                onOpen={() => onOpen(item)}
              />
            );
          })}
          <Button
            fullWidth
            startIcon={<AddIcon />}
            onClick={() => onAdd(slot)}
            sx={{ py: 1.25, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}
          >
            Add food
          </Button>
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
  const markEaten = useMarkMealPlanEaten();
  const reopenEntry = useReopenMealPlanEntry();

  const currentMonday = startOfWeekIso(todayIso());
  const activeDate = day >= weekStart && day <= addDays(weekStart, 6) ? day : weekStart;

  function goToWeek(start: string) {
    void navigate({
      to: '/meal-plan/$weekStart/$day',
      params: { weekStart: start, day: defaultDayFor(start) },
    });
  }

  function goToDay(date: string) {
    void navigate({ to: '/meal-plan/$weekStart/$day', params: { weekStart, day: date } });
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  const selectedDay = week.data?.days.find((candidate) => candidate.date === activeDate) ?? week.data?.days[0];

  function addFood(slot: MealSlot) {
    if (!selectedDay) return;
    setAdding({ key: crypto.randomUUID(), date: selectedDay.date, slot });
  }

  function openItem(slot: MealSlot, item: MealItem) {
    if (!selectedDay) return;
    setEditing({ key: crypto.randomUUID(), date: selectedDay.date, slot, item });
  }

  async function toggleItem(item: MealItem) {
    if (item.kind !== 'planned') return;
    const key = item.component_id;
    setToggling(key);
    try {
      if (item.status === 'planned') {
        await markEaten.mutateAsync({
          id: item.entry_id,
          revision: item.revision,
          body: {
            consumed_on: todayIso(),
            consumed_at: new Date().toISOString(),
            components: [{ component_id: item.component_id, amount: item.amount }],
          },
        });
      } else {
        await reopenEntry.mutateAsync({ id: item.entry_id, revision: item.revision });
      }
    } finally {
      setToggling(null);
    }
  }

  return (
    <Box>
      <PageHeader
        title="Meals"
        actions={
          selectedDay ? (
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => addFood(DEFAULT_SLOT)}>
              Add food
            </Button>
          ) : null
        }
      />

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
                kcal={slotView.items.length > 0 ? (slotView.nutrition.nutrition.energy_kcal ?? null) : null}
                toggling={toggling}
                onToggle={toggleItem}
                onOpen={(item) => openItem(slotView.slot, item)}
                onAdd={addFood}
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

      {editing ? (
        <EditFoodDialog
          key={editing.key}
          open
          onClose={() => setEditing(null)}
          memberId={memberId}
          date={editing.date}
          slot={editing.slot}
          item={editing.item}
        />
      ) : null}
    </Box>
  );
}
