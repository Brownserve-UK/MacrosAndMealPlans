import CalendarMonthIcon from '@mui/icons-material/CalendarMonthOutlined';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { addDays, formatDayLabel, formatFullDate } from './date';

export function DayNav({ date, onChange }: { date: string; onChange: (next: string) => void }) {
  return (
    <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', justifyContent: 'center' }}>
      <IconButton
        onClick={() => onChange(addDays(date, -1))}
        aria-label="Previous day"
        size="small"
        sx={{ border: '1px solid', borderColor: 'divider' }}
      >
        <ChevronLeftIcon />
      </IconButton>

      <Stack sx={{ alignItems: 'center', minWidth: { xs: 168, sm: 208 } }} spacing={0.25}>
        <Typography variant="h3">{formatDayLabel(date)}</Typography>
        <Box
          component="label"
          sx={{
            position: 'relative',
            display: 'inline-flex',
            alignItems: 'center',
            gap: 0.5,
            color: 'text.secondary',
            cursor: 'pointer',
            '&:hover': { color: 'primary.main' },
            '&:has(input:focus-visible)': {
              borderRadius: 1,
              outline: '2px solid',
              outlineColor: 'primary.main',
              outlineOffset: 2,
            },
          }}
        >
          <CalendarMonthIcon sx={{ fontSize: 15 }} />
          <Typography variant="caption">{formatFullDate(date)}</Typography>
          <Box
            component="input"
            aria-label="Choose day"
            type="date"
            value={date}
            onChange={(e) => e.target.value && onChange(e.target.value)}
            sx={{ position: 'absolute', inset: 0, opacity: 0, width: '100%', cursor: 'pointer' }}
          />
        </Box>
      </Stack>

      <IconButton
        onClick={() => onChange(addDays(date, 1))}
        aria-label="Next day"
        size="small"
        sx={{ border: '1px solid', borderColor: 'divider' }}
      >
        <ChevronRightIcon />
      </IconButton>
    </Stack>
  );
}
