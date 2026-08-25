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
  { key: 'sugar_g', label: 'Sugars', unit: 'g' },
  { key: 'saturated_fat_g', label: 'Saturates', unit: 'g' },
  { key: 'fibre_g', label: 'Fibre', unit: 'g' },
  { key: 'salt_g', label: 'Salt', unit: 'g' },
  { key: 'cholesterol_mg', label: 'Cholesterol', unit: 'mg' },
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

const STATUS_COLOUR: Record<Status, string> = {
  good: 'success.main',
  under: 'warning.main',
  over: 'error.main',
  neutral: 'text.secondary',
};

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

type Comparison = { colour: string | undefined; hint: string | null };

function compare(
  key: string,
  value: number | null | undefined,
  scope: ScopeSummary,
  directions: Directions,
  unit: string,
): Comparison {
  if (scope.notEnoughData?.includes(key)) {
    return { colour: undefined, hint: 'Not enough data' };
  }
  const target = scope.target?.[key as keyof NutritionGoals];
  if (target == null) return { colour: undefined, hint: null };
  const status = classify(value, target, directions[key]);
  return { colour: STATUS_COLOUR[status], hint: `of ${Math.round(target)} ${unit}` };
}

function macroShares(summary: Summary) {
  const macros = MACROS.map((macro) => {
    const grams = summary.nutrition[macro.key];
    return { ...macro, grams, amount: Math.max(grams ?? 0, 0) };
  });
  const total = macros.reduce((sum, macro) => sum + macro.amount, 0);
  return macros.map((macro) => ({
    ...macro,
    share: total > 0 ? (macro.amount / total) * 100 : 0,
  }));
}

function SplitStat({
  label,
  value,
  eaten,
  colour,
}: {
  label: string;
  value: number | null | undefined;
  eaten: boolean;
  colour?: string;
}) {
  return (
    <Box>
      <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center', mb: 0.25 }}>
        <Box
          aria-hidden
          sx={{
            width: 8,
            height: 8,
            flexShrink: 0,
            borderRadius: '50%',
            backgroundColor: eaten ? 'text.primary' : 'transparent',
            border: '1.5px solid',
            borderColor: eaten ? 'text.primary' : 'text.disabled',
          }}
        />
        <Typography variant="caption" color="text.secondary">
          {label}
        </Typography>
      </Stack>
      <Typography
        className="numeral"
        sx={{
          fontWeight: eaten ? 700 : 500,
          color: colour ?? (eaten ? 'text.primary' : 'text.secondary'),
        }}
      >
        <MaybeNumber value={value} fractionDigits={0} />{' '}
        <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
          kcal
        </Box>
      </Typography>
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
  const macros = macroShares(projected);
  const incomplete = projected.unknown_count + projected.partial_count;
  const energy = compare('energy_kcal', projected.nutrition.energy_kcal, scope, directions, 'kcal');
  const eatenEnergy = compare('energy_kcal', actual.nutrition.energy_kcal, scope, directions, 'kcal');
  const splitLabel = macros.some((macro) => macro.share > 0)
    ? `${label} macro split: ${macros
        .map((macro) => `${macro.label} ${macro.share.toFixed(1)}%`)
        .join(', ')}`
    : `${label} macro split is not available`;

  return (
    <Box>
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', sm: 'minmax(170px, 0.55fr) 1.6fr' },
          alignItems: 'center',
          columnGap: { xs: 1.5, sm: 5 },
          rowGap: 2.5,
        }}
      >
        <Box>
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.25 }}>
            Projected energy
          </Typography>
          <Typography
            className="numeral"
            sx={{
              fontSize: { xs: '2.25rem', sm: '2.75rem' },
              fontWeight: 600,
              lineHeight: 1,
              letterSpacing: '-0.025em',
              color: energy.colour,
            }}
          >
            <MaybeNumber value={projected.nutrition.energy_kcal} fractionDigits={0} />{' '}
            <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.8rem' }}>
              kcal
            </Box>
          </Typography>
          {energy.hint ? (
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.25 }}>
              {energy.hint}
            </Typography>
          ) : null}
        </Box>

        <Box sx={{ minWidth: 0 }}>
          <Box
            role="img"
            aria-label={splitLabel}
            sx={{
              display: 'flex',
              gap: '2px',
              height: 8,
              mb: 1.25,
              overflow: 'hidden',
              borderRadius: 999,
              backgroundColor: 'action.hover',
            }}
          >
            {macros.map((macro) =>
              macro.share > 0 ? (
                <Box key={macro.key} sx={{ width: `${macro.share}%`, backgroundColor: macro.colour }} />
              ) : null,
            )}
          </Box>
          <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 2 }}>
            {macros.map((macro) => {
              const comparison = compare(macro.key, macro.grams, scope, directions, macro.unit);
              return (
                <Box key={macro.key} sx={{ minWidth: 0 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.75, mb: 0.25 }}>
                    <Box
                      aria-hidden
                      sx={{ width: 7, height: 7, flexShrink: 0, borderRadius: '50%', backgroundColor: macro.colour }}
                    />
                    <Typography variant="caption" color="text.secondary" noWrap>
                      {macro.label}
                    </Typography>
                  </Box>
                  <Typography className="numeral" sx={{ fontWeight: 600, color: comparison.colour }}>
                    <MaybeNumber value={macro.grams} />{' '}
                    <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                      g
                    </Box>
                  </Typography>
                  {comparison.hint ? (
                    <Typography variant="caption" color="text.secondary" noWrap>
                      {comparison.hint}
                    </Typography>
                  ) : null}
                </Box>
              );
            })}
          </Box>
        </Box>
      </Box>

      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
          gap: 3,
          mt: 2.5,
          pt: 2.25,
          borderTop: '1px solid',
          borderColor: 'divider',
        }}
      >
        <SplitStat label="Eaten" value={actual.nutrition.energy_kcal} eaten colour={eatenEnergy.colour} />
        <SplitStat label="Still planned" value={remaining.nutrition.energy_kcal} eaten={false} />
      </Box>

      <Box
        component="dl"
        sx={{
          m: 0,
          display: 'grid',
          gridTemplateColumns: { xs: 'repeat(2, 1fr)', sm: 'repeat(5, 1fr)' },
          gap: { xs: 2, sm: 3 },
          mt: 2.5,
          pt: 2.25,
          borderTop: '1px solid',
          borderColor: 'divider',
        }}
      >
        {DETAILS.map(({ key, label: detailLabel, unit }) => {
          const value = projected.nutrition[key];
          const comparison = compare(key, value, scope, directions, unit);
          return (
            <Box key={key} component="div">
              <Typography component="dt" variant="caption" color="text.secondary" sx={{ mb: 0.25 }}>
                {detailLabel}
              </Typography>
              <Typography
                component="dd"
                className="numeral"
                sx={{ m: 0, fontWeight: 600, color: comparison.colour }}
              >
                <MaybeNumber value={value} />{' '}
                <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                  {unit}
                </Box>
              </Typography>
              {comparison.hint ? (
                <Typography variant="caption" color="text.secondary" noWrap>
                  {comparison.hint}
                </Typography>
              ) : null}
            </Box>
          );
        })}
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
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', mb: 2.25 }}
      >
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
