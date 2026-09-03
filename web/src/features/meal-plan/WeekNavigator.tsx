import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { addDays, formatWeekRange, parseIsoDate } from './date';

export type WeekNavigatorDay = {
  date: string;
  itemCount: number;
};

function longDayName(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function WeekDayRail({
  days,
  value,
  onChange,
}: {
  days: WeekNavigatorDay[];
  value: string;
  onChange: (date: string) => void;
}) {
  return (
    <Box sx={{ display: 'flex', overflowX: 'auto', p: 0.75 }}>
      {days.map((day) => {
        const selected = day.date === value;
        const date = parseIsoDate(day.date);
        return (
          <ButtonBase
            key={day.date}
            onClick={() => onChange(day.date)}
            aria-pressed={selected}
            aria-label={`${longDayName(day.date)}, ${day.itemCount} ${day.itemCount === 1 ? 'item' : 'items'}`}
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
                  backgroundColor: day.itemCount > 0 ? 'primary.main' : 'transparent',
                }}
              />
            </Stack>
          </ButtonBase>
        );
      })}
    </Box>
  );
}

export function WeekNavigator({
  weekStart,
  days,
  selectedDate,
  currentMonday,
  onWeekChange,
  onDayChange,
}: {
  weekStart: string;
  days: WeekNavigatorDay[];
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
