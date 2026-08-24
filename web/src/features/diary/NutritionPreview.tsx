import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { Nutrition } from '../../api/client';
import { MaybeNumber } from '../../components/Unknown';

const MACROS = [
  { key: 'protein_g', label: 'Protein' },
  { key: 'carbohydrate_g', label: 'Carbs' },
  { key: 'fat_g', label: 'Fat' },
] as const;

export function NutritionPreview({ nutrition }: { nutrition: Nutrition }) {
  return (
    <Box sx={{ p: 2, borderRadius: 2, backgroundColor: 'action.hover' }}>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        For this amount
      </Typography>
      <Stack
        direction="row"
        spacing={{ xs: 2, sm: 3 }}
        sx={{ alignItems: 'baseline', flexWrap: 'wrap', rowGap: 1 }}
      >
        <Typography className="numeral" sx={{ fontSize: '1.25rem', fontWeight: 600 }}>
          <MaybeNumber value={nutrition.energy_kcal} fractionDigits={0} />{' '}
          <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
            kcal
          </Box>
        </Typography>
        {MACROS.map(({ key, label }) => (
          <Typography key={key} className="numeral" variant="body2" sx={{ fontWeight: 500 }}>
            <Box component="span" sx={{ color: 'text.secondary', mr: 0.75 }}>
              {label}
            </Box>
            <MaybeNumber value={nutrition[key]} suffix="g" />
          </Typography>
        ))}
      </Stack>
    </Box>
  );
}
