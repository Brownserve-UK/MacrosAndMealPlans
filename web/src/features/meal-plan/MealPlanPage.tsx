import AddIcon from '@mui/icons-material/AddOutlined';
import CheckCircleIcon from '@mui/icons-material/CheckCircleOutlineOutlined';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import RestaurantIcon from '@mui/icons-material/RestaurantOutlined';
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
import type { DiaryEntry, MealPlanDay, MealPlanEntry, MealSlot } from '../../api/client';
import { useDiaryDay, useMealPlanWeek } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import { addDays, formatWeekRange, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { EditLoggedFoodDialog } from './EditLoggedFoodDialog';
import { formatAmount } from './format';
import { LogFoodDialog } from './LogFoodDialog';
import { MealPlanEntryDialog, SLOTS } from './MealPlanEntryDialog';
import { DayWeekNutrition } from './NutritionSummary';

const DEFAULT_SLOT: MealSlot = SLOTS[0]?.value ?? 'breakfast';

type Selection = {
  key: string;
  date: string;
  slot: MealSlot;
  entry: MealPlanEntry | null;
};

function longDayName(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function entryTitle(entry: MealPlanEntry) {
  return entry.components.map((component) => component.product_name).join(' + ');
}

function EntryRow({ entry, divided, onClick }: { entry: MealPlanEntry; divided: boolean; onClick: () => void }) {
  const nutrition = entry.actual ?? entry.planned;
  const title = entryTitle(entry);
  const detail = [
    entry.planned_time?.slice(0, 5),
    entry.components.map((component) => formatAmount(component.amount)).join(' · '),
    entry.status === 'eaten' ? 'Eaten' : null,
    entry.status === 'not_eaten' ? 'Not eaten' : null,
  ]
    .filter(Boolean)
    .join(' · ');

  return (
    <ButtonBase
      onClick={onClick}
      aria-label={`Open ${title}`}
      sx={{
        display: 'flex',
        width: '100%',
        alignItems: 'center',
        gap: { xs: 1.5, sm: 2 },
        px: { xs: 2, sm: 2.5 },
        py: 1.75,
        textAlign: 'left',
        borderTop: divided ? '1px solid' : 'none',
        borderColor: 'divider',
        opacity: entry.status === 'not_eaten' ? 0.55 : 1,
        transition: 'background-color 120ms ease',
        '&:hover': { backgroundColor: 'action.hover' },
        '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main', outlineOffset: -2 },
      }}
    >
      <InitialsAvatar name={title} size={44} />
      <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
        <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center', minWidth: 0 }}>
          <Typography
            variant="subtitle1"
            sx={{
              minWidth: 0,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              textDecoration: entry.status === 'not_eaten' ? 'line-through' : 'none',
            }}
          >
            {title}
          </Typography>
          {entry.status === 'eaten' ? (
            <CheckCircleIcon titleAccess="Eaten" color="success" sx={{ flexShrink: 0, fontSize: 17 }} />
          ) : null}
        </Stack>
        {detail ? (
          <Typography variant="caption" color="text.secondary">
            {detail}
          </Typography>
        ) : null}
      </Stack>
      {entry.needs_attention ? (
        <Chip
          size="small"
          variant="outlined"
          icon={<WarningAmberIcon />}
          label="Needs attention"
          sx={{ display: { xs: 'none', sm: 'inline-flex' }, flexShrink: 0 }}
        />
      ) : null}
      {entry.status !== 'not_eaten' ? (
        <Typography className="numeral" variant="body2" sx={{ flexShrink: 0, fontWeight: 600 }}>
          <MaybeNumber value={nutrition.nutrition.energy_kcal} fractionDigits={0} />{' '}
          <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
            kcal
          </Box>
        </Typography>
      ) : null}
      <ChevronRightIcon sx={{ color: 'text.disabled', fontSize: 20, flexShrink: 0 }} />
    </ButtonBase>
  );
}

function LoggedRow({ entry, divided, onClick }: { entry: DiaryEntry; divided: boolean; onClick: () => void }) {
  return (
    <ButtonBase
      onClick={onClick}
      aria-label={`Edit ${entry.product_name}`}
      sx={{
        display: 'flex',
        width: '100%',
        alignItems: 'center',
        gap: { xs: 1.5, sm: 2 },
        px: { xs: 2, sm: 2.5 },
        py: 1.75,
        textAlign: 'left',
        borderTop: divided ? '1px solid' : 'none',
        borderColor: 'divider',
        transition: 'background-color 120ms ease',
        '&:hover': { backgroundColor: 'action.hover' },
        '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main', outlineOffset: -2 },
      }}
    >
      <InitialsAvatar name={entry.product_name} size={44} />
      <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
        <Typography variant="subtitle1" sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {entry.product_name}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {formatAmount(entry.amount)}
        </Typography>
      </Stack>
      {entry.quality === 'unknown' ? (
        <Chip size="small" variant="outlined" label="No nutrition" sx={{ flexShrink: 0 }} />
      ) : (
        <Typography className="numeral" variant="body2" sx={{ flexShrink: 0, fontWeight: 600 }}>
          <MaybeNumber value={entry.nutrition.energy_kcal} fractionDigits={0} />{' '}
          <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
            kcal
          </Box>
        </Typography>
      )}
      <ChevronRightIcon sx={{ color: 'text.disabled', fontSize: 20, flexShrink: 0 }} />
    </ButtonBase>
  );
}

function SlotSection({
  slot,
  label,
  entries,
  onOpen,
  onAdd,
}: {
  slot: MealSlot;
  label: string;
  entries: MealPlanEntry[];
  onOpen: (entry: MealPlanEntry) => void;
  onAdd: (slot: MealSlot) => void;
}) {
  return (
    <Box component="section" aria-label={label}>
      <Typography variant="overline" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        {label}
      </Typography>
      {entries.length === 0 ? (
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
          Add {label.toLowerCase()}
        </Button>
      ) : (
        <Paper sx={{ overflow: 'hidden' }}>
          {entries.map((entry, index) => (
            <EntryRow key={entry.id} entry={entry} divided={index > 0} onClick={() => onOpen(entry)} />
          ))}
          <Button
            fullWidth
            startIcon={<AddIcon />}
            onClick={() => onAdd(slot)}
            sx={{ py: 1.25, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}
          >
            Add to {label.toLowerCase()}
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
        const mealCount = day.entries.length;
        return (
          <ButtonBase
            key={day.date}
            onClick={() => onChange(day.date)}
            aria-pressed={selected}
            aria-label={`${longDayName(day.date)}, ${mealCount} ${mealCount === 1 ? 'meal' : 'meals'}`}
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
                  backgroundColor: mealCount > 0 ? 'primary.main' : 'transparent',
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

export function MealPlanPage({ weekStart }: { weekStart: string }) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const week = useMealPlanWeek(weekStart);
  const [selectedDate, setSelectedDate] = useState(() => {
    const today = todayIso();
    return today >= weekStart && today <= addDays(weekStart, 6) ? today : weekStart;
  });
  const [selection, setSelection] = useState<Selection | null>(null);
  const [logging, setLogging] = useState(false);
  const [editingRecord, setEditingRecord] = useState<DiaryEntry | null>(null);

  const currentMonday = startOfWeekIso(todayIso());
  const activeDate =
    selectedDate >= weekStart && selectedDate <= addDays(weekStart, 6) ? selectedDate : weekStart;
  const diary = useDiaryDay(memberId, activeDate);

  function goTo(start: string) {
    void navigate({ to: '/meal-plan/$weekStart', params: { weekStart: start } });
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  const selectedDay = week.data?.days.find((day) => day.date === activeDate) ?? week.data?.days[0];
  const unplanned = (diary.data?.entries ?? []).filter((entry) => !entry.meal_plan_entry_id);

  function planMeal(slot: MealSlot) {
    if (!selectedDay) return;
    setSelection({ key: crypto.randomUUID(), date: selectedDay.date, slot, entry: null });
  }

  return (
    <Box sx={{ maxWidth: 980, mx: 'auto' }}>
      <PageHeader
        title="Meal plan"
        actions={
          selectedDay ? (
            <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
              <Button variant="outlined" startIcon={<RestaurantIcon />} onClick={() => setLogging(true)}>
                Log food
              </Button>
              <Button variant="contained" startIcon={<AddIcon />} onClick={() => planMeal(DEFAULT_SLOT)}>
                Plan meal
              </Button>
            </Stack>
          ) : null
        }
      />

      {week.data ? (
        <WeekNavigator
          weekStart={weekStart}
          days={week.data.days}
          selectedDate={activeDate}
          currentMonday={currentMonday}
          onWeekChange={goTo}
          onDayChange={setSelectedDate}
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
              day={{
                actual: selectedDay.actual,
                remaining: selectedDay.remaining_planned,
                projected: selectedDay.projected,
              }}
              week={{
                actual: week.data.actual,
                remaining: week.data.remaining_planned,
                projected: week.data.projected,
              }}
            />

            {SLOTS.map((candidate) => (
              <SlotSection
                key={candidate.value}
                slot={candidate.value}
                label={candidate.label}
                entries={selectedDay.entries.filter((entry) => entry.slot === candidate.value)}
                onOpen={(entry) =>
                  setSelection({ key: entry.id, date: selectedDay.date, slot: entry.slot, entry })
                }
                onAdd={planMeal}
              />
            ))}

            {unplanned.length > 0 ? (
              <Box component="section" aria-label="Logged unplanned food">
                <Typography variant="overline" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
                  Also eaten (unplanned)
                </Typography>
                <Paper sx={{ overflow: 'hidden' }}>
                  {unplanned.map((entry, index) => (
                    <LoggedRow
                      key={entry.id}
                      entry={entry}
                      divided={index > 0}
                      onClick={() => setEditingRecord(entry)}
                    />
                  ))}
                  <Button
                    fullWidth
                    startIcon={<AddIcon />}
                    onClick={() => setLogging(true)}
                    sx={{ py: 1.25, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}
                  >
                    Log more food
                  </Button>
                </Paper>
              </Box>
            ) : null}
          </Stack>
        </Box>
      ) : null}

      {selection ? (
        <MealPlanEntryDialog
          key={selection.key}
          open
          onClose={() => setSelection(null)}
          date={selection.date}
          slot={selection.slot}
          entry={selection.entry}
        />
      ) : null}

      {selectedDay ? (
        <LogFoodDialog
          open={logging}
          onClose={() => setLogging(false)}
          memberId={memberId}
          date={selectedDay.date}
        />
      ) : null}

      {editingRecord ? (
        <EditLoggedFoodDialog open record={editingRecord} onClose={() => setEditingRecord(null)} />
      ) : null}
    </Box>
  );
}
