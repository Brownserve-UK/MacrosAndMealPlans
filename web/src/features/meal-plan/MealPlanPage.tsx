import AddIcon from '@mui/icons-material/AddOutlined';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonthOutlined';
import CheckCircleIcon from '@mui/icons-material/CheckCircleOutlineOutlined';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import type { MealPlanDay, MealPlanEntry, MealSlot } from '../../api/client';
import { useMealPlanMembers, useMealPlanWeek } from '../../api/queries';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import {
  addDays,
  formatWeekRange,
  parseIsoDate,
  startOfWeekIso,
  todayIso,
} from '../diary/date';
import { formatAmount } from '../diary/format';
import { MealPlanEntryDialog, SLOTS } from './MealPlanEntryDialog';
import { MealPlanSummary } from './MealPlanSummary';

type Selection = {
  key: string;
  date: string;
  slot: MealSlot;
  entry: MealPlanEntry | null;
};

type MemberOption = {
  id: string;
  display_name: string;
};

function longDayName(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function slotLabel(slot: MealSlot) {
  return SLOTS.find((candidate) => candidate.value === slot)?.label ?? slot;
}

function entryTitle(entry: MealPlanEntry) {
  return entry.components.map((component) => component.product_name).join(' + ');
}

function EntryRow({
  entry,
  divided,
  onClick,
}: {
  entry: MealPlanEntry;
  divided: boolean;
  onClick: () => void;
}) {
  const nutrition = entry.actual ?? entry.planned;
  const title = entryTitle(entry);
  const detail = [
    slotLabel(entry.slot),
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
        '&:focus-visible': {
          outline: '2px solid',
          outlineColor: 'primary.main',
          outlineOffset: -2,
        },
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
        <Typography variant="caption" color="text.secondary">
          {detail}
        </Typography>
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

function EmptyPlan({ onAdd }: { onAdd: () => void }) {
  return (
    <Paper sx={{ px: 3, py: { xs: 5, sm: 6 }, textAlign: 'center' }}>
      <Box
        sx={{
          width: 52,
          height: 52,
          mx: 'auto',
          mb: 2,
          display: 'grid',
          placeItems: 'center',
          borderRadius: '50%',
          color: 'primary.main',
          backgroundColor: 'action.selected',
        }}
      >
        <CalendarMonthIcon />
      </Box>
      <Typography variant="h3" sx={{ mb: 0.75 }}>
        Nothing planned
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
        Add the first meal for this day.
      </Typography>
      <Button variant="contained" startIcon={<AddIcon />} onClick={onAdd}>
        Plan meal
      </Button>
    </Paper>
  );
}

function DaySchedule({
  day,
  onSelect,
  onAdd,
}: {
  day: MealPlanDay;
  onSelect: (selection: Selection) => void;
  onAdd: () => void;
}) {
  if (day.entries.length === 0) return <EmptyPlan onAdd={onAdd} />;

  return (
    <Paper sx={{ overflow: 'hidden' }}>
      {day.entries.map((entry, index) => (
        <EntryRow
          key={entry.id}
          entry={entry}
          divided={index > 0}
          onClick={() =>
            onSelect({ key: entry.id, date: day.date, slot: entry.slot, entry })
          }
        />
      ))}
      <Button
        fullWidth
        startIcon={<AddIcon />}
        onClick={onAdd}
        sx={{ py: 1.5, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}
      >
        Add another meal
      </Button>
    </Paper>
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
  members,
  memberId,
  onWeekChange,
  onDayChange,
  onMemberChange,
}: {
  weekStart: string;
  days: MealPlanDay[];
  selectedDate: string;
  currentMonday: string;
  members: MemberOption[];
  memberId: string;
  onWeekChange: (week: string) => void;
  onDayChange: (date: string) => void;
  onMemberChange: (member: string) => void;
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
        {members.length > 1 ? (
          <TextField
            select
            label="Plan for"
            value={memberId}
            onChange={(event) => onMemberChange(event.target.value)}
            size="small"
            sx={{ justifySelf: { sm: 'end' }, minWidth: { xs: '100%', sm: 180 } }}
          >
            {members.map((member) => (
              <MenuItem key={member.id} value={member.id}>
                {member.display_name}
              </MenuItem>
            ))}
          </TextField>
        ) : null}
      </Box>
      <Divider />
      <WeekDayRail days={days} value={selectedDate} onChange={onDayChange} />
    </Paper>
  );
}

export function MealPlanPage({ memberId, weekStart }: { memberId: string; weekStart: string }) {
  const navigate = useNavigate();
  const members = useMealPlanMembers();
  const week = useMealPlanWeek(memberId, weekStart);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [selectedDate, setSelectedDate] = useState(() => {
    const today = todayIso();
    return today >= weekStart && today <= addDays(weekStart, 6) ? today : weekStart;
  });

  function goTo(start: string, member = memberId) {
    void navigate({
      to: '/meal-plan/$memberId/$weekStart',
      params: { memberId: member, weekStart: start },
    });
  }

  if (members.isError) return <ErrorState error={members.error} onRetry={() => members.refetch()} />;
  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  const currentMonday = startOfWeekIso(todayIso());
  const activeDate =
    selectedDate >= weekStart && selectedDate <= addDays(weekStart, 6) ? selectedDate : weekStart;
  const selectedDay = week.data?.days.find((day) => day.date === activeDate) ?? week.data?.days[0];
  const weekHasMeals = week.data?.days.some((day) => day.entries.length > 0) ?? false;
  const defaultSlot =
    SLOTS.find(({ value }) => !selectedDay?.entries.some((entry) => entry.slot === value))?.value ??
    'snacks';

  function addMeal() {
    if (!selectedDay) return;
    setSelection({
      key: crypto.randomUUID(),
      date: selectedDay.date,
      slot: defaultSlot,
      entry: null,
    });
  }

  return (
    <Box sx={{ maxWidth: 980, mx: 'auto' }}>
      <PageHeader
        title="Meal plan"
        subtitle="Plan meals ahead, then confirm what was actually eaten."
        actions={
          selectedDay ? (
            <Button variant="contained" startIcon={<AddIcon />} onClick={addMeal}>
              Plan meal
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
          members={members.data ?? []}
          memberId={memberId}
          onWeekChange={goTo}
          onDayChange={setSelectedDate}
          onMemberChange={(member) => goTo(weekStart, member)}
        />
      ) : null}

      {week.isLoading ? <Loading label="Loading week" /> : null}
      {week.data && selectedDay ? (
        <Stack spacing={3.5}>
          {weekHasMeals ? (
            <Box component="section" aria-labelledby="week-nutrition-heading">
              <Typography id="week-nutrition-heading" variant="h3" sx={{ mb: 1.25 }}>
                Week nutrition
              </Typography>
              <MealPlanSummary
                actual={week.data.actual}
                remaining={week.data.remaining_planned}
                projected={week.data.projected}
              />
            </Box>
          ) : null}

          <Box component="section" aria-labelledby="day-plan-heading">
            <Stack
              direction={{ xs: 'column', sm: 'row' }}
              spacing={{ xs: 0.25, sm: 2 }}
              sx={{
                mb: 1.25,
                alignItems: { xs: 'flex-start', sm: 'baseline' },
                justifyContent: 'space-between',
              }}
            >
              <Typography id="day-plan-heading" variant="h2">
                {longDayName(selectedDay.date)}
              </Typography>
              {selectedDay.entries.length > 0 ? (
                <Typography variant="caption" color="text.secondary">
                  {selectedDay.entries.length === 1
                    ? '1 meal'
                    : `${selectedDay.entries.length} meals`}{' '}
                  · <MaybeNumber value={selectedDay.projected.nutrition.energy_kcal} fractionDigits={0} /> kcal projected
                </Typography>
              ) : null}
            </Stack>
            <DaySchedule day={selectedDay} onSelect={setSelection} onAdd={addMeal} />
          </Box>
        </Stack>
      ) : null}

      {selection ? (
        <MealPlanEntryDialog
          key={selection.key}
          open
          onClose={() => setSelection(null)}
          memberId={memberId}
          date={selection.date}
          slot={selection.slot}
          entry={selection.entry}
        />
      ) : null}
    </Box>
  );
}
