import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { MealPlanSummary as Summary } from '../../api/client';
import { MaybeNumber } from '../../components/Unknown';

function EnergyValue({ value }: { value: number | null | undefined }) {
  return (
    <Typography className="numeral" sx={{ fontSize: '1.3rem', fontWeight: 650 }}>
      <MaybeNumber value={value} fractionDigits={0} />
      <Box component="span" sx={{ ml: 0.5, fontSize: '0.72rem', color: 'text.secondary' }}>
        kcal
      </Box>
    </Typography>
  );
}

function Macro({ label, value }: { label: string; value: number | null | undefined }) {
  return (
    <Box>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography className="numeral" variant="body2" sx={{ mt: 0.25, fontWeight: 650 }}>
        <MaybeNumber value={value} />g
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
  const incomplete = projected.unknown_count + projected.partial_count;

  return (
    <Paper sx={{ p: { xs: 2.5, sm: 3 } }}>
      <Typography variant="overline" color="text.secondary">
        Week outlook
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
        Projected energy
      </Typography>
      <Typography
        className="numeral"
        sx={{ mt: 0.25, fontSize: '2.35rem', fontWeight: 650, lineHeight: 1.2 }}
      >
        <MaybeNumber value={projected.nutrition.energy_kcal} fractionDigits={0} />
        <Box component="span" sx={{ ml: 0.75, fontSize: '0.8rem', color: 'text.secondary' }}>
          kcal
        </Box>
      </Typography>

      <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2, mt: 3 }}>
        <Box>
          <Typography variant="caption" color="text.secondary">
            Eaten
          </Typography>
          <EnergyValue value={actual.nutrition.energy_kcal} />
        </Box>
        <Box>
          <Typography variant="caption" color="text.secondary">
            Still planned
          </Typography>
          <EnergyValue value={remaining.nutrition.energy_kcal} />
        </Box>
      </Box>

      <Divider sx={{ my: 2.5 }} />

      <Typography variant="caption" color="text.secondary">
        Projected macros
      </Typography>
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 2, mt: 1 }}>
        <Macro label="Protein" value={projected.nutrition.protein_g} />
        <Macro label="Carbs" value={projected.nutrition.carbohydrate_g} />
        <Macro label="Fat" value={projected.nutrition.fat_g} />
      </Box>

      {incomplete > 0 ? (
        <Stack direction="row" spacing={1} sx={{ alignItems: 'flex-start', mt: 2.5, color: 'warning.main' }}>
          <WarningAmberIcon sx={{ mt: 0.1, fontSize: 18 }} />
          <Typography variant="caption">
            {incomplete} {incomplete === 1 ? 'meal has' : 'meals have'} incomplete nutrition.
          </Typography>
        </Stack>
      ) : null}
    </Paper>
  );
}
