import ButtonBase from '@mui/material/ButtonBase';
import LinearProgress from '@mui/material/LinearProgress';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { PlannerWeek, TargetDirection } from '../../api/client';

type DayNutrition = PlannerWeek['days'][number];

function progressColour(value: number, target: number, direction?: TargetDirection | null) {
  if (direction === 'at_most' && value > target) return 'error';
  if (direction === 'at_least' && value < target) return 'warning';
  if (direction === 'around' && Math.abs(value - target) > target * 0.1) return 'warning';
  return 'success';
}

export function CompactNutrition({ day, onClick }: { day: DayNutrition; onClick: () => void }) {
  const projected = day.projected.nutrition.energy_kcal;
  const target = day.target?.energy_kcal;
  const hasProgress = projected != null && target != null && target > 0;
  const value = hasProgress ? projected : 0;
  const label = projected == null
    ? 'Projected calories unavailable'
    : target == null
      ? `${Math.round(projected).toLocaleString('en-GB')} kcal projected`
      : `${Math.round(projected).toLocaleString('en-GB')} kcal projected of ${Math.round(target).toLocaleString('en-GB')}`;

  return (
    <ButtonBase onClick={onClick} sx={{ display: 'block', width: '100%', textAlign: 'left', borderRadius: 1.25, mb: 3 }}>
      <Stack spacing={0.75} sx={{ width: '100%' }}>
        <Typography variant="body2" className="numeral">{label}</Typography>
        <LinearProgress
          variant="determinate"
          value={hasProgress ? Math.min((value / target) * 100, 100) : 0}
          color={hasProgress ? progressColour(value, target, day.calorie_direction) : 'inherit'}
          aria-label={label}
          sx={{ height: 4, borderRadius: 999 }}
        />
      </Stack>
    </ButtonBase>
  );
}
