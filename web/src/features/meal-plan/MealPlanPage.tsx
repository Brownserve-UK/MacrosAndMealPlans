import AddIcon from '@mui/icons-material/AddOutlined';
import CheckCircleIcon from '@mui/icons-material/CheckCircleOutlineOutlined';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
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

function EntryRow({ entry, onClick }: { entry: MealPlanEntry; onClick: () => void }) {
  const nutrition = entry.actual ?? entry.planned;
  const detail = [
    entry.planned_time?.slice(0, 5),
    entry.components.map((component) => formatAmount(component.amount)).join(' · '),
  ]
    .filter(Boolean)
    .join(' · ');

  return (
    <ButtonBase
      onClick={onClick}
      aria-label={`Open ${entryTitle(entry)}`}
      sx={{
        width: '100%',
        display: 'grid',
        gridTemplateColumns: 'minmax(0, 1fr) auto',
        gap: 2,
        alignItems: 'center',
        py: 1.5,
        textAlign: 'left',
        borderRadius: 1.5,
        opacity: entry.status === 'not_eaten' ? 0.55 : 1,
        '&:hover': { backgroundColor: 'action.hover' },
        '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main' },
      }}
    >
      <Box sx={{ minWidth: 0 }}>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <Typography
            sx={{
              minWidth: 0,
              fontWeight: 650,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              textDecoration: entry.status === 'not_eaten' ? 'line-through' : 'none',
            }}
          >
            {entryTitle(entry)}
          </Typography>
          {entry.status === 'eaten' ? (
            <CheckCircleIcon color="success" sx={{ flexShrink: 0, fontSize: 17 }} />
          ) : null}
          {entry.needs_attention ? (
            <WarningAmberIcon
              titleAccess="Needs attention"
              color="warning"
              sx={{ flexShrink: 0, fontSize: 17 }}
            />
          ) : null}
        </Stack>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.25 }}>
          {detail}
          {entry.status === 'not_eaten' ? `${detail ? ' · ' : ''}Not eaten` : ''}
          {entry.status === 'eaten' ? `${detail ? ' · ' : ''}Eaten` : ''}
        </Typography>
      </Box>
      {entry.status !== 'not_eaten' ? (
        <Typography className="numeral" variant="body2" sx={{ fontWeight: 650 }}>
          <MaybeNumber value={nutrition.nutrition.energy_kcal} fractionDigits={0} /> kcal
        </Typography>
      ) : null}
    </ButtonBase>
  );
}

function DaySchedule({ day, onSelect }: { day: MealPlanDay; onSelect: (selection: Selection) => void }) {
  return (
    <Paper sx={{ overflow: 'hidden' }}>
      {SLOTS.map(({ value }, index) => {
        const entries = day.entries.filter((entry) => entry.slot === value);
        const label = slotLabel(value);
        return (
          <Box key={value}>
            {index > 0 ? <Divider /> : null}
            <Box
              sx={{
                display: 'grid',
                gridTemplateColumns: { xs: '1fr', sm: '112px minmax(0, 1fr)' },
                gap: { xs: 0.5, sm: 3 },
                px: { xs: 2, sm: 3 },
                py: { xs: 2, sm: 2.25 },
              }}
            >
              <Typography variant="subtitle2" sx={{ pt: { sm: 1.5 } }}>
                {label}
              </Typography>
              {entries.length > 0 ? (
                <Stack divider={<Divider />}>
                  {entries.map((entry) => (
                    <EntryRow
                      key={entry.id}
                      entry={entry}
                      onClick={() => onSelect({ key: entry.id, date: day.date, slot: value, entry })}
                    />
                  ))}
                </Stack>
              ) : (
                <ButtonBase
                  onClick={() =>
                    onSelect({ key: crypto.randomUUID(), date: day.date, slot: value, entry: null })
                  }
                  sx={{
                    justifySelf: 'start',
                    py: 1.25,
                    color: 'text.secondary',
                    borderRadius: 1.5,
                    '&:hover': { color: 'primary.main' },
                    '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main' },
                  }}
                >
                  <AddIcon sx={{ mr: 0.75, fontSize: 18 }} />
                  <Typography variant="body2">Plan {label.toLowerCase()}</Typography>
                </ButtonBase>
              )}
            </Box>
          </Box>
        );
      })}
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
    <Paper sx={{ display: 'flex', overflowX: 'auto', p: 0.75 }}>
      {days.map((day) => {
        const selected = day.date === value;
        const date = parseIsoDate(day.date);
        const mealCount = day.entries.length;
        return (
          <ButtonBase
            key={day.date}
            onClick={() => onChange(day.date)}
            aria-pressed={selected}
            sx={{
              position: 'relative',
              minWidth: { xs: 72, sm: 0 },
              flex: { sm: 1 },
              px: 1,
              py: 1.25,
              borderRadius: 1.5,
              backgroundColor: selected ? 'action.selected' : 'transparent',
              '&:hover': { backgroundColor: 'action.hover' },
              '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main' },
              '&::after': selected
                ? {
                    content: '""',
                    position: 'absolute',
                    right: 14,
                    bottom: 4,
                    left: 14,
                    height: 2,
                    borderRadius: 2,
                    backgroundColor: 'primary.main',
                  }
                : undefined,
            }}
          >
            <Stack spacing={0.25} sx={{ alignItems: 'center' }}>
              <Typography variant="caption" color={selected ? 'text.primary' : 'text.secondary'}>
                {date.toLocaleDateString('en-GB', { weekday: 'short' })}
              </Typography>
              <Typography className="numeral" sx={{ fontSize: '1.1rem', fontWeight: 650 }}>
                {date.getDate()}
              </Typography>
              <Typography variant="caption" color="text.secondary" sx={{ minHeight: 18 }}>
                {mealCount > 0 ? `${mealCount} ${mealCount === 1 ? 'meal' : 'meals'}` : ''}
              </Typography>
            </Stack>
          </ButtonBase>
        );
      })}
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
  const defaultSlot =
    SLOTS.find(({ value }) => !selectedDay?.entries.some((entry) => entry.slot === value))?.value ??
    'snacks';

  return (
    <>
      <PageHeader
        title="Meal plan"
        subtitle="A clear view of what is planned, and what was actually eaten."
      />

      <Stack spacing={3}>
        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={2}
          sx={{ justifyContent: 'space-between', alignItems: { sm: 'center' } }}
        >
          <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
            <IconButton aria-label="Previous week" onClick={() => goTo(addDays(weekStart, -7))}>
              <ChevronLeftIcon />
            </IconButton>
            <Typography variant="subtitle1" sx={{ minWidth: { sm: 180 }, textAlign: 'center' }}>
              {formatWeekRange(weekStart)}
            </Typography>
            <IconButton aria-label="Next week" onClick={() => goTo(addDays(weekStart, 7))}>
              <ChevronRightIcon />
            </IconButton>
            {weekStart !== currentMonday ? (
              <Button size="small" onClick={() => goTo(currentMonday)}>
                This week
              </Button>
            ) : null}
          </Stack>
          <TextField
            select
            label="Plan for"
            value={memberId}
            onChange={(event) => goTo(weekStart, event.target.value)}
            size="small"
            sx={{ minWidth: { xs: '100%', sm: 180 } }}
          >
            {(members.data ?? []).map((member) => (
              <MenuItem key={member.id} value={member.id}>
                {member.display_name}
              </MenuItem>
            ))}
          </TextField>
        </Stack>

        {week.isLoading ? <Loading label="Loading week" /> : null}
        {week.data && selectedDay ? (
          <>
            <WeekDayRail days={week.data.days} value={activeDate} onChange={setSelectedDate} />
            <Box
              sx={{
                display: 'grid',
                gridTemplateColumns: {
                  xs: 'minmax(0, 1fr)',
                  md: 'minmax(0, 1.75fr) minmax(280px, 0.65fr)',
                },
                gap: 3,
                alignItems: 'start',
              }}
            >
              <Stack spacing={2}>
                <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
                  <Box>
                    <Typography variant="h3">{longDayName(selectedDay.date)}</Typography>
                    <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
                      <MaybeNumber value={selectedDay.projected.nutrition.energy_kcal} fractionDigits={0} /> kcal projected
                    </Typography>
                  </Box>
                  <Button
                    variant="contained"
                    startIcon={<AddIcon />}
                    onClick={() =>
                      setSelection({
                        key: crypto.randomUUID(),
                        date: selectedDay.date,
                        slot: defaultSlot,
                        entry: null,
                      })
                    }
                  >
                    Plan meal
                  </Button>
                </Stack>
                <DaySchedule day={selectedDay} onSelect={setSelection} />
              </Stack>

              <Box sx={{ position: { md: 'sticky' }, top: { md: 24 } }}>
                <MealPlanSummary
                  actual={week.data.actual}
                  remaining={week.data.remaining_planned}
                  projected={week.data.projected}
                />
              </Box>
            </Box>
          </>
        ) : null}
      </Stack>

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
    </>
  );
}
