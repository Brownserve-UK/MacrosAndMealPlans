import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import type { Nutrition } from '../../api/client';
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

function macroShares(nutrition: Nutrition | undefined) {
  const macros = MACROS.map((macro) => {
    const grams = nutrition?.[macro.key];
    return { ...macro, grams, amount: Math.max(grams ?? 0, 0) };
  });
  const total = macros.reduce((sum, macro) => sum + macro.amount, 0);
  return macros.map((macro) => ({
    ...macro,
    share: total > 0 ? (macro.amount / total) * 100 : 0,
  }));
}

export function DailyNutritionSummary({
  nutrition,
  incompleteCount,
}: {
  nutrition: Nutrition | undefined;
  incompleteCount: number;
}) {
  const macros = macroShares(nutrition);
  const splitLabel = macros.some((macro) => macro.share > 0)
    ? `Macro split: ${macros
        .map((macro) => `${macro.label} ${macro.share.toFixed(1)}%`)
        .join(', ')}`
    : 'Macro split is not available';

  return (
    <Paper sx={{ px: { xs: 2, sm: 3 }, py: { xs: 2.25, sm: 2.75 } }}>
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', sm: 'minmax(150px, 0.55fr) 1.6fr' },
          gridTemplateRows: { xs: 'auto auto', sm: 'auto' },
          alignItems: 'center',
          columnGap: { xs: 1.5, sm: 5 },
          rowGap: 2.5,
        }}
      >
        <Box sx={{ gridColumn: 1, gridRow: 1 }}>
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.25 }}>
            Energy
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
            <MaybeNumber value={nutrition?.energy_kcal} fractionDigits={0} />{' '}
            <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.8rem' }}>
              kcal
            </Box>
          </Typography>
        </Box>

        <Box
          sx={{
            gridColumn: { xs: '1 / -1', sm: 2 },
            gridRow: { xs: 2, sm: 1 },
            minWidth: 0,
          }}
        >
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.75 }}>
            Macro split
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
        {DETAILS.map(({ key, label, unit }) => (
          <Box key={key} component="div">
            <Typography
              component="dt"
              variant="caption"
              color="text.secondary"
              sx={{ mb: 0.25 }}
            >
              {label}
            </Typography>
            <Typography component="dd" className="numeral" sx={{ m: 0, fontWeight: 600 }}>
              <MaybeNumber value={nutrition?.[key]} />{' '}
              <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
                {unit}
              </Box>
            </Typography>
          </Box>
        ))}
      </Box>

      {incompleteCount > 0 ? (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1.75 }}>
          {incompleteCount === 1
            ? '1 item has incomplete nutrition'
            : `${incompleteCount} items have incomplete nutrition`}
        </Typography>
      ) : null}
    </Paper>
  );
}
