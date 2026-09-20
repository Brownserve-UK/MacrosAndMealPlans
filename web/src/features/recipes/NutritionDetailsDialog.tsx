import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { RecipeNutrition } from '../../api/client';

const HINT: Record<Exclude<RecipeNutrition['quality'], 'known'>, string> = {
  estimated:
    'Estimated from typical products for each ingredient. Actual values depend on the products you use.',
  partial:
    'Estimated from typical products for each ingredient. Actual values depend on the products you use. Some ingredients have no nutrition data yet.',
  unknown: 'No nutrition data for these ingredients yet.',
};

const GAP_REASON: Record<RecipeNutrition['gaps'][number]['reason'], string> = {
  unmatched: 'Not matched to an ingredient',
  no_data: 'No nutrition data',
  incomplete: 'Incomplete nutrition data',
};

type NutritionKey =
  | 'energy_kcal'
  | 'fat_g'
  | 'saturated_fat_g'
  | 'carbohydrate_g'
  | 'sugar_g'
  | 'fibre_g'
  | 'protein_g'
  | 'salt_g'
  | 'cholesterol_mg';

const BREAKDOWN: { key: NutritionKey; label: string; unit: string; indent?: boolean }[] = [
  { key: 'energy_kcal', label: 'Energy', unit: 'kcal' },
  { key: 'fat_g', label: 'Fat', unit: 'g' },
  { key: 'saturated_fat_g', label: 'of which saturates', unit: 'g', indent: true },
  { key: 'carbohydrate_g', label: 'Carbohydrate', unit: 'g' },
  { key: 'sugar_g', label: 'of which sugars', unit: 'g', indent: true },
  { key: 'fibre_g', label: 'Fibre', unit: 'g' },
  { key: 'protein_g', label: 'Protein', unit: 'g' },
  { key: 'salt_g', label: 'Salt', unit: 'g' },
  { key: 'cholesterol_mg', label: 'Cholesterol', unit: 'mg' },
];

function round(value: number): number {
  return Math.round(value * 10) / 10;
}

export function NutritionDetailsDialog({
  data,
  onClose,
  onResolve,
}: {
  data: RecipeNutrition;
  onClose: () => void;
  onResolve: (componentId: string, text: string) => void;
}) {
  const extra = Object.entries(data.nutrition.extra ?? {});
  return (
    <Dialog open onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>Nutrition details</DialogTitle>
      <DialogContent>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>Per serving</Typography>
        <Stack spacing={1.25}>
          {BREAKDOWN.map((row) => {
            const value = data.nutrition[row.key];
            return (
              <Stack
                key={row.key}
                direction="row"
                sx={{ justifyContent: 'space-between', pl: row.indent ? 2 : 0 }}
              >
                <Typography variant="body2" color="text.secondary">{row.label}</Typography>
                <Typography
                  variant="body2"
                  className="numeral"
                  sx={value == null ? { color: 'warning.main' } : undefined}
                >
                  {value == null ? 'Unknown' : `${round(value)} ${row.unit}`}
                </Typography>
              </Stack>
            );
          })}
          {extra.map(([name, value]) => (
            <Stack key={name} direction="row" sx={{ justifyContent: 'space-between' }}>
              <Typography variant="body2" color="text.secondary">{name}</Typography>
              <Typography variant="body2" className="numeral">{round(value)}</Typography>
            </Stack>
          ))}
        </Stack>

        {data.quality !== 'known' ? (
          <Typography variant="body2" color="text.secondary" sx={{ mt: 2.5 }}>
            {HINT[data.quality]}
          </Typography>
        ) : null}

        {data.gaps.length > 0 ? (
          <>
            <Divider sx={{ my: 2.5 }} />
            <Typography variant="h3" sx={{ mb: 1.5 }}>Missing data</Typography>
            <Stack spacing={1.5} divider={<Divider flexItem />}>
              {data.gaps.map((gap, index) => (
                <Stack
                  key={gap.component_id ?? `${gap.name}-${index}`}
                  direction="row"
                  sx={{ justifyContent: 'space-between', alignItems: 'center', gap: 2 }}
                >
                  <Stack spacing={0.25}>
                    <Typography variant="body2">{gap.name}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      {GAP_REASON[gap.reason]}
                    </Typography>
                  </Stack>
                  {gap.reason === 'unmatched' && gap.component_id ? (
                    <Button size="small" onClick={() => onResolve(gap.component_id!, gap.name)}>
                      Match
                    </Button>
                  ) : null}
                </Stack>
              ))}
            </Stack>
          </>
        ) : null}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
}
