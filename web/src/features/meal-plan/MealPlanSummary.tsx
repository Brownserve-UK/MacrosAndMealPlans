import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import type { MealPlanSummary as Summary } from '../../api/client';
import { MaybeNumber } from '../../components/Unknown';

const MACROS = [
  { key: 'protein_g', label: 'Protein', colour: 'primary.main' },
  { key: 'carbohydrate_g', label: 'Carbs', colour: 'info.main' },
  { key: 'fat_g', label: 'Fat', colour: 'secondary.main' },
] as const;

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

function EnergyDetail({
  label,
  value,
}: {
  label: string;
  value: number | null | undefined;
}) {
  return (
    <Box>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography className="numeral" sx={{ mt: 0.25, fontWeight: 600 }}>
        <MaybeNumber value={value} fractionDigits={0} />{' '}
        <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
          kcal
        </Box>
      </Typography>
    </Box>
  );
}

export function MealPlanSummary({
  actual,
  remaining,
  projected,
}: {
  actual: Summary;
  remaining: Summary;
  projected: Summary;
}) {
  const macros = macroShares(projected);
  const incomplete = projected.unknown_count + projected.partial_count;
  const splitLabel = macros.some((macro) => macro.share > 0)
    ? `Projected macro split: ${macros
        .map((macro) => `${macro.label} ${macro.share.toFixed(1)}%`)
        .join(', ')}`
    : 'Projected macro split is not available';

  return (
    <Paper sx={{ px: { xs: 2, sm: 3 }, py: { xs: 2.25, sm: 2.75 } }}>
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
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.75 }}>
            Projected macro split
          </Typography>
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
                <Box
                  key={macro.key}
                  sx={{ width: `${macro.share}%`, backgroundColor: macro.colour }}
                />
              ) : null,
            )}
          </Box>
          <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 2 }}>
            {macros.map((macro) => (
              <Box key={macro.key} sx={{ minWidth: 0 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.75, mb: 0.25 }}>
                  <Box
                    aria-hidden
                    sx={{
                      width: 7,
                      height: 7,
                      flexShrink: 0,
                      borderRadius: '50%',
                      backgroundColor: macro.colour,
                    }}
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
        <EnergyDetail label="Already eaten" value={actual.nutrition.energy_kcal} />
        <EnergyDetail label="Still planned" value={remaining.nutrition.energy_kcal} />
      </Box>

      {incomplete > 0 ? (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1.75 }}>
          {incomplete === 1
            ? '1 meal has incomplete nutrition'
            : `${incomplete} meals have incomplete nutrition`}
        </Typography>
      ) : null}
    </Paper>
  );
}
