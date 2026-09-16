import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import type { ReactNode } from 'react';
import { PageHeader } from '../../components/PageHeader';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { defaultDayFor, fullDayLabel, startOfWeekIso, todayIso } from './date';
import { WeekNavigator, type WeekNavigatorDay } from './WeekNavigator';

export function PlannerShell({
  weekStart,
  activeDate,
  dayCounts,
  headerActions,
  nutrition,
  error,
  onDismissError,
  children,
}: {
  weekStart: string;
  activeDate: string;
  dayCounts: WeekNavigatorDay[] | null;
  headerActions?: ReactNode;
  nutrition?: ReactNode;
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
    });
  }

  function goToDay(date: string) {
    void navigate({
      to: '/planner/$weekStart/$day',
      params: { weekStart, day: date },
    });
  }

  return (
    <Box>
      <PageHeader
        title="Planner"
        actions={headerActions}
      />
      {error ? <Alert severity="error" onClose={onDismissError} sx={{ mb: 2 }}>{error}</Alert> : null}
      {nutrition}
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
