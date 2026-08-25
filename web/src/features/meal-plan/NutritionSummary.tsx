import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { MealPlanSummary as Summary } from '../../api/client';
import { MaybeNumber } from '../../components/Unknown';

const MACROS = [
  { key: 'protein_g', label: 'Protein', colour: 'primary.main' },
  { key: 'carbohydrate_g', label: 'Carbs', colour: 'info.main' },
  { key: 'fat_g', label: 'Fat', colour: 'secondary.main' },
] as const;

const DETAILS = [
  { key: 'sugar_g', label: 'Sugars', unit: 'g' },
  { key: 'saturated_fat_g', label: 'Saturates', unit: 'g' },
  { key: 'fibre_g', label: 'Fibre', unit: 'g' },
  { key: 'salt_g', label: 'Salt', unit: 'g' },
  { key: 'cholesterol_mg', label: 'Cholesterol', unit: 'mg' },
] as const;

export type ScopeSummary = {
  actual: Summary;
  remaining: Summary;
  projected: Summary;
};

type Scope = 'day' | 'week';

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
}: {
  label: string;
  value: number | null | undefined;
  eaten: boolean;
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
        sx={{ fontWeight: eaten ? 700 : 500, color: eaten ? 'text.primary' : 'text.secondary' }}
      >
        <MaybeNumber value={value} fractionDigits={0} />{' '}
        <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
          kcal
        </Box>
      </Typography>
    </Box>
  );
}

function NutritionBlock({ scope, label }: { scope: ScopeSummary; label: string }) {
  const { actual, remaining, projected } = scope;
  const macros = macroShares(projected);
  const incomplete = projected.unknown_count + projected.partial_count;
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
            }}
          >
            <MaybeNumber value={projected.nutrition.energy_kcal} fractionDigits={0} />{' '}
            <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.8rem' }}>
              kcal
            </Box>
          </Typography>
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
            {macros.map((macro) => (
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
                <Typography className="numeral" sx={{ fontWeight: 600 }}>
                  <MaybeNumber value={macro.grams} />{' '}
                  <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                    g
                  </Box>
                </Typography>
              </Box>
            ))}
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
        <SplitStat label="Eaten" value={actual.nutrition.energy_kcal} eaten />
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
        {DETAILS.map(({ key, label: detailLabel, unit }) => (
          <Box key={key} component="div">
            <Typography component="dt" variant="caption" color="text.secondary" sx={{ mb: 0.25 }}>
              {detailLabel}
            </Typography>
            <Typography component="dd" className="numeral" sx={{ m: 0, fontWeight: 600 }}>
              <MaybeNumber value={projected.nutrition[key]} />{' '}
              <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                {unit}
              </Box>
            </Typography>
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

export function DayWeekNutrition({ day, week }: { day: ScopeSummary; week: ScopeSummary }) {
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
      <NutritionBlock scope={selected} label={label} />
    </Paper>
  );
}
