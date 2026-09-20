import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import type { ShoppingOpportunity } from '../../api/client';
import { formatDayLabel, formatFullDate } from '../meal-plan/date';
import { plannedCaption } from './forMeals';
import { sectionLabel } from './sections';

export function TripCard({
  opportunity,
  count,
  planned,
  sections,
  imminent,
  onChange,
}: {
  opportunity: ShoppingOpportunity;
  count?: number;
  planned?: number | null;
  sections?: string[];
  imminent?: boolean;
  onChange: () => void;
}) {
  const heading = imminent ? whenItIs(opportunity.date) : formatDayLabel(opportunity.date);
  const beneath = [
    imminent ? formatFullDate(opportunity.date) : null,
    count == null ? null : count === 1 ? '1 thing' : `${count} things`,
    count == null || planned == null ? null : plannedCaption(planned),
    opportunity.state === 'moved' && opportunity.generated_for
      ? `Moved from ${formatDayLabel(opportunity.generated_for)}`
      : null,
    opportunity.state === 'one_off' ? 'An extra trip' : null,
  ].filter((part) => part != null);

  return (
    <Paper variant="outlined" sx={{ p: 2.5 }}>
      <Stack
        direction={{ xs: 'column', sm: 'row' }}
        spacing={2}
        sx={{ alignItems: { sm: 'flex-start' } }}
      >
        <Box sx={{ flexGrow: 1, minWidth: 0 }}>
          <Typography variant={imminent ? 'h2' : 'subtitle1'}>{heading}</Typography>
          <Typography variant="body2" color="text.secondary" className="numeral">
            {beneath.join(' · ')}
          </Typography>
          {sections && sections.length > 0 ? (
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              {sections.map((section) => sectionLabel(section as never)).join(', ')}
            </Typography>
          ) : null}
        </Box>
        <Box sx={{ flexShrink: 0 }}>
          <Link to="/shopping/$date" params={{ date: opportunity.date }} className="app-link">
            <Button variant={imminent ? 'outlined' : 'text'} component="span">
              Open list
            </Button>
          </Link>
        </Box>
      </Stack>

      <Box sx={{ mt: 1.5, ml: -1.5 }}>
        <Button size="small" onClick={onChange}>
          Change
        </Button>
      </Box>
    </Paper>
  );
}

function whenItIs(iso: string): string {
  const label = formatDayLabel(iso);
  if (label === 'Today' || label === 'Tomorrow') return label;
  const weekday = new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', { weekday: 'long' });
  return `This ${weekday}`;
}
