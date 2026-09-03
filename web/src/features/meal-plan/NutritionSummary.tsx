import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type {
  MealPlanSummary as Summary,
  NutritionGoals,
  TargetDirection,
} from '../../api/client';
import { MaybeNumber } from '../../components/Unknown';

const MACROS = [
  { key: 'protein_g', label: 'Protein', colour: 'primary.main', unit: 'g' },
  { key: 'carbohydrate_g', label: 'Carbs', colour: 'info.main', unit: 'g' },
  { key: 'fat_g', label: 'Fat', colour: 'secondary.main', unit: 'g' },
] as const;

const DETAILS = [
  { key: 'sugar_g', label: 'Sugars', colour: 'secondary.main', unit: 'g' },
  { key: 'saturated_fat_g', label: 'Saturates', colour: 'secondary.main', unit: 'g' },
  { key: 'fibre_g', label: 'Fibre', colour: 'primary.main', unit: 'g' },
  { key: 'salt_g', label: 'Salt', colour: 'info.main', unit: 'g' },
  { key: 'cholesterol_mg', label: 'Cholesterol', colour: 'info.main', unit: 'mg' },
] as const;

export type Directions = Record<string, TargetDirection>;

export type ScopeSummary = {
  actual: Summary;
  remaining: Summary;
  projected: Summary;
  target?: NutritionGoals | null;
  notEnoughData?: readonly string[];
};

type Scope = 'day' | 'week';
type Status = 'good' | 'under' | 'over' | 'neutral';

function classify(
  value: number | null | undefined,
  target: number | null | undefined,
  direction: TargetDirection | undefined,
): Status {
  if (value == null || target == null || direction == null) return 'neutral';
  switch (direction) {
    case 'at_most':
      return value <= target ? 'good' : 'over';
    case 'at_least':
      return value >= target ? 'good' : 'under';
    case 'around': {
      const tolerance = Math.abs(target) * 0.1;
      if (Math.abs(value - target) <= tolerance) return 'good';
      return value > target ? 'over' : 'under';
    }
    default:
      return 'neutral';
  }
}

function statusColour(status: Status, direction: TargetDirection | undefined) {
  if (status === 'over') return 'error.main';
  if (status === 'good' && direction === 'at_least') return 'success.main';
  return 'text.primary';
}

function formatTarget(target: number, unit: string) {
  return `${Math.round(target).toLocaleString('en-GB')} ${unit}`;
}

function energyStatus(
  value: number | null | undefined,
  target: number | null | undefined,
  direction: TargetDirection | undefined,
) {
  if (value == null || target == null || direction == null) {
    return { amount: value, label: 'Projected', colour: 'text.primary' };
  }

  const difference = Math.round(Math.abs(target - value));
  const status = classify(value, target, direction);
  if (direction === 'at_most') {
    return value > target
      ? { amount: difference, label: 'Over', colour: 'error.main' }
      : { amount: difference, label: 'Under', colour: 'success.main' };
  }
  if (direction === 'at_least') {
    return value >= target
      ? { amount: difference, label: 'Above', colour: 'success.main' }
      : { amount: difference, label: 'To target', colour: 'warning.main' };
  }
  return {
    amount: status === 'good' ? 0 : difference,
    label: status === 'good' ? 'On target' : value > target ? 'Over' : 'To target',
    colour: status === 'good' ? 'success.main' : 'warning.main',
  };
}

function EnergyDial({ scope, label, directions }: { scope: ScopeSummary; label: string; directions: Directions }) {
  const value = scope.projected.nutrition.energy_kcal;
  const target = scope.target?.energy_kcal;
  const direction = directions.energy_kcal;
  const incomplete = scope.notEnoughData?.includes('energy_kcal') ?? false;
  const status = incomplete ? 'neutral' : classify(value, target, direction);
  const display = incomplete
    ? { amount: value, label: 'Projected', colour: 'text.primary' }
    : energyStatus(value, target, direction);
  const progress =
    !incomplete && value != null && target != null && target > 0 ? Math.min((value / target) * 100, 100) : 0;
  const overflow =
    !incomplete && direction != null && value != null && target != null && target > 0 && value > target
      ? Math.min(Math.max(((value - target) / target) * 100, 3), 16)
      : 0;
  const overflowColour = direction === 'at_least' ? 'success.main' : 'error.main';
  const ariaValue = value == null ? 'unknown' : `${Math.round(value)} kcal`;
  const ariaTarget = target == null ? 'without a target' : `against a target of ${Math.round(target)} kcal`;
  const displayLength = display.amount == null ? 1 : Math.round(Math.abs(display.amount)).toLocaleString('en-GB').length;
  const displayFontSize =
    displayLength >= 7
      ? { xs: '1.45rem', sm: '1.75rem' }
      : displayLength >= 5
        ? { xs: '1.7rem', sm: '2rem' }
        : { xs: '2rem', sm: '2.4rem' };

  return (
    <Box sx={{ textAlign: 'center', minWidth: 0 }}>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
        {target == null ? `${label} energy` : `Energy target ${formatTarget(target, 'kcal')}`}
      </Typography>
      <Box
        role="img"
        aria-label={`${label} projected energy is ${ariaValue} ${ariaTarget}`}
        sx={{ position: 'relative', width: '100%', maxWidth: 250, mx: 'auto' }}
      >
        <Box
          component="svg"
          viewBox="0 0 200 116"
          aria-hidden
          sx={{ display: 'block', width: '100%', overflow: 'visible' }}
        >
          <Box
            component="path"
            d="M 20 100 A 80 80 0 0 1 180 100"
            pathLength="100"
            sx={{ color: 'action.hover', fill: 'none', stroke: 'currentColor', strokeWidth: 14, strokeLinecap: 'round' }}
          />
          <Box
            component="path"
            d="M 20 100 A 80 80 0 0 1 180 100"
            pathLength="100"
            sx={{
              fill: 'none',
              color: status === 'over' ? 'success.main' : display.colour,
              stroke: 'currentColor',
              strokeWidth: 14,
              strokeLinecap: 'round',
              strokeDasharray: `${progress} 100`,
            }}
          />
          {overflow > 0 ? (
            <Box
              component="path"
              d="M 20 100 A 80 80 0 0 1 180 100"
              pathLength="100"
              sx={{
                fill: 'none',
                color: overflowColour,
                stroke: 'currentColor',
                strokeWidth: 14,
                strokeLinecap: 'round',
                strokeDasharray: `${overflow} 100`,
                strokeDashoffset: overflow - 100,
              }}
            />
          ) : null}
        </Box>
        <Box sx={{ position: 'absolute', inset: '48% 0 auto', transform: 'translateY(-10%)' }}>
          <Typography
            className="numeral"
            sx={{
              color: display.colour,
              fontSize: displayFontSize,
              fontWeight: 600,
              lineHeight: 1,
            }}
          >
            <MaybeNumber value={display.amount} fractionDigits={0} />
          </Typography>
          <Typography variant="body2" sx={{ color: display.colour, lineHeight: 1.3 }}>
            {display.label}
          </Typography>
        </Box>
      </Box>
      <Typography variant="caption" color="text.secondary">
        <Box component="span" className="numeral" sx={{ color: 'text.primary', fontWeight: 650 }}>
          <MaybeNumber value={value} fractionDigits={0} />
        </Box>{' '}
        kcal projected
      </Typography>
      {incomplete ? (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
          Not enough data
        </Typography>
      ) : null}
    </Box>
  );
}

function EnergyStat({
  label,
  value,
  primary,
}: {
  label: string;
  value: number | null | undefined;
  primary?: boolean;
}) {
  return (
    <Box sx={{ textAlign: 'center', minWidth: 0 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography
        className="numeral"
        sx={{
          fontSize: { xs: '1.35rem', sm: '1.6rem' },
          fontWeight: primary ? 700 : 500,
          lineHeight: 1.25,
        }}
      >
        <MaybeNumber value={value} fractionDigits={0} />
      </Typography>
      <Typography variant="caption" color="text.secondary">
        kcal
      </Typography>
    </Box>
  );
}

function NutrientProgress({
  metricKey,
  label,
  unit,
  colour,
  value,
  scope,
  directions,
}: {
  metricKey: string;
  label: string;
  unit: string;
  colour: string;
  value: number | null | undefined;
  scope: ScopeSummary;
  directions: Directions;
}) {
  const target = scope.target?.[metricKey as keyof NutritionGoals];
  const direction = directions[metricKey];
  const incomplete = scope.notEnoughData?.includes(metricKey) ?? false;
  const status = incomplete ? 'neutral' : classify(value, target, direction);
  const progress = value != null && target != null && target > 0 ? Math.min((value / target) * 100, 100) : 0;
  const overflow =
    !incomplete && direction != null && value != null && target != null && target > 0 && value > target
      ? Math.min(Math.max(((value - target) / target) * 100, 3), 16)
      : 0;
  const overflowColour = direction === 'at_least' ? 'success.main' : 'error.main';
  const valueText = value == null ? 'Unknown' : `${value.toFixed(1)} ${unit}`;
  const targetText = target == null ? 'No target' : `target ${Math.round(target)} ${unit}`;

  return (
    <Box sx={{ minWidth: 0 }}>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
        {label}
      </Typography>
      <Box
        role="progressbar"
        aria-label={`${label}: ${valueText}, ${targetText}`}
        aria-valuemin={0}
        aria-valuemax={target == null ? undefined : Math.max(target, value ?? 0)}
        aria-valuenow={value ?? undefined}
        sx={{
          position: 'relative',
          height: 8,
          overflow: 'hidden',
          borderRadius: 999,
          backgroundColor: 'action.hover',
        }}
      >
        <Box sx={{ width: `${progress}%`, height: '100%', borderRadius: 999, backgroundColor: colour }} />
        {overflow > 0 ? (
          <Box
            sx={{
              position: 'absolute',
              top: 0,
              right: 0,
              width: `${overflow}%`,
              height: '100%',
              backgroundColor: overflowColour,
            }}
          />
        ) : null}
      </Box>
      <Typography
        className="numeral"
        sx={{ mt: 0.6, fontSize: '0.9rem', fontWeight: 600, color: statusColour(status, direction) }}
      >
        <MaybeNumber value={value} />{' '}
        <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem', fontWeight: 400 }}>
          {target == null ? unit : `/ ${Math.round(target)} ${unit}`}
        </Box>
      </Typography>
      {incomplete ? (
        <Typography variant="caption" color="text.secondary" noWrap>
          Not enough data
        </Typography>
      ) : null}
    </Box>
  );
}

function NutritionBlock({
  scope,
  label,
  directions,
}: {
  scope: ScopeSummary;
  label: string;
  directions: Directions;
}) {
  const { actual, remaining, projected } = scope;
  const incomplete = projected.unknown_count + projected.partial_count;

  return (
    <Box>
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: 'repeat(2, minmax(0, 1fr))', sm: '1fr minmax(220px, 1.4fr) 1fr' },
          alignItems: 'center',
          columnGap: { xs: 2, sm: 3 },
          rowGap: 1.5,
        }}
      >
        <Box sx={{ gridColumn: { xs: '1 / -1', sm: 2 }, gridRow: 1 }}>
          <EnergyDial scope={scope} label={label} directions={directions} />
        </Box>
        <Box sx={{ gridColumn: { xs: 1, sm: 1 }, gridRow: { xs: 2, sm: 1 } }}>
          <EnergyStat label="Eaten" value={actual.nutrition.energy_kcal} primary />
        </Box>
        <Box sx={{ gridColumn: { xs: 2, sm: 3 }, gridRow: { xs: 2, sm: 1 } }}>
          <EnergyStat label="Still planned" value={remaining.nutrition.energy_kcal} />
        </Box>
      </Box>

      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', sm: 'repeat(3, minmax(0, 1fr))' },
          gap: { xs: 2, sm: 3 },
          mt: 2.5,
          pt: 2.25,
          borderTop: '1px solid',
          borderColor: 'divider',
        }}
      >
        {MACROS.map((metric) => (
          <NutrientProgress
            key={metric.key}
            metricKey={metric.key}
            label={metric.label}
            unit={metric.unit}
            colour={metric.colour}
            value={projected.nutrition[metric.key]}
            scope={scope}
            directions={directions}
          />
        ))}
      </Box>

      <Box
        component="dl"
        sx={{
          m: 0,
          display: 'grid',
          gridTemplateColumns: { xs: 'repeat(2, minmax(0, 1fr))', sm: 'repeat(5, minmax(0, 1fr))' },
          gap: { xs: 2, sm: 3 },
          mt: 2.5,
          pt: 2.25,
          borderTop: '1px solid',
          borderColor: 'divider',
        }}
      >
        {DETAILS.map((metric) => (
          <Box key={metric.key} component="div">
            <NutrientProgress
              metricKey={metric.key}
              label={metric.label}
              unit={metric.unit}
              colour={metric.colour}
              value={projected.nutrition[metric.key]}
              scope={scope}
              directions={directions}
            />
          </Box>
        ))}
      </Box>

      {incomplete > 0 ? (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1.75 }}>
          {incomplete === 1
            ? '1 meal has incomplete nutrition'
            : `${incomplete} meals have incomplete nutrition`}
        </Typography>
      ) : null}
    </Box>
  );
}

export function DayWeekNutrition({
  day,
  week,
  directions,
}: {
  day: ScopeSummary;
  week: ScopeSummary;
  directions: Directions;
}) {
  const [scope, setScope] = useState<Scope>('day');
  const selected = scope === 'day' ? day : week;
  const label = scope === 'day' ? 'Day' : 'Week';

  return (
    <Paper sx={{ px: { xs: 2, sm: 3 }, py: { xs: 2.25, sm: 2.75 } }}>
      <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', mb: 1.5 }}>
        <Typography variant="overline" color="text.secondary">
          Nutrition
        </Typography>
        <ToggleButtonGroup
          exclusive
          size="small"
          value={scope}
          aria-label="Nutrition scope"
          onChange={(_event, next: Scope | null) => {
            if (next) setScope(next);
          }}
        >
          <ToggleButton value="day">Day</ToggleButton>
          <ToggleButton value="week">Week</ToggleButton>
        </ToggleButtonGroup>
      </Stack>
      <NutritionBlock scope={selected} label={label} directions={directions} />
    </Paper>
  );
}
