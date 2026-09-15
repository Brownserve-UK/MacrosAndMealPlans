import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import type { ReactNode } from 'react';
import { PageHeader } from '../../components/PageHeader';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { defaultDayFor, parseIsoDate, startOfWeekIso, todayIso } from './date';
import { PlannerLens, type Lens } from './PlannerLens';
import { WeekNavigator, type WeekNavigatorDay } from './WeekNavigator';

function fullDayLabel(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long' });
}

export function PlannerShell({
  lens,
  showLens,
  onLensChange,
  weekStart,
  activeDate,
  dayCounts,
  headerActions,
  error,
  onDismissError,
  children,
}: {
  lens: Lens;
  showLens: boolean;
  onLensChange: (lens: Lens) => void;
  weekStart: string;
  activeDate: string;
  dayCounts: WeekNavigatorDay[] | null;
  headerActions?: ReactNode;
  error: string | null;
  onDismissError: () => void;
  children: ReactNode;
}) {
  const navigate = useNavigate();
  const timeZone = useHouseholdTimeZone();
  function goToWeek(start: string) {
    void navigate({
      to: '/planner/$weekStart/$day',
      params: { weekStart: start, day: defaultDayFor(start, timeZone) },
      search: { lens },
    });
  }

  function goToDay(date: string) {
    void navigate({
      to: '/planner/$weekStart/$day',
      params: { weekStart, day: date },
      search: { lens },
    });
  }

  return (
    <Box>
      <PageHeader
        title="Planner"
        actions={
          <>
            <PlannerLens lens={lens} onChange={onLensChange} show={showLens} />
            {headerActions}
          </>
        }
      />
      {error ? <Alert severity="error" onClose={onDismissError} sx={{ mb: 2 }}>{error}</Alert> : null}
      {dayCounts ? (
        <WeekNavigator
          weekStart={weekStart}
          days={dayCounts}
          selectedDate={activeDate}
          currentMonday={startOfWeekIso(todayIso(timeZone))}
          onWeekChange={goToWeek}
          onDayChange={goToDay}
        />
      ) : null}
      <Typography variant="h2" sx={{ mb: 2 }}>{fullDayLabel(activeDate)}</Typography>
      {children}
    </Box>
  );
}
