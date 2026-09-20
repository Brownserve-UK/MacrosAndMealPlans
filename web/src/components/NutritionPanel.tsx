import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { Nutrition } from '../api/client';
import { derivedBasis, dimensionOf, type PackContext } from './NutritionFields';
import { displayUnit } from './UnitSelect';
import { MaybeNumber } from './Unknown';

const MACROS = [
  { key: 'protein_g', label: 'Protein' },
  { key: 'carbohydrate_g', label: 'Carbs' },
  { key: 'fat_g', label: 'Fat' },
] as const;

const REST = [
  { key: 'sugar_g', label: 'of which sugars', unit: 'g', indent: true },
  { key: 'saturated_fat_g', label: 'of which saturates', unit: 'g', indent: true },
  { key: 'fibre_g', label: 'Fibre', unit: 'g', indent: false },
  { key: 'salt_g', label: 'Salt', unit: 'g', indent: false },
  { key: 'cholesterol_mg', label: 'Cholesterol', unit: 'mg', indent: false },
] as const;

const ALL_KEYS = [
  'energy_kcal',
  ...MACROS.map((m) => m.key),
  ...REST.map((r) => r.key),
] as const;

export function isNutritionUnknown(nutrition: Nutrition | undefined): boolean {
  if (!nutrition) return true;
  return ALL_KEYS.every((key) => nutrition[key] === null || nutrition[key] === undefined);
}

function basisLabel(nutrition: Nutrition | undefined, pack: PackContext | undefined): string | null {
  const basis = nutrition?.basis;
  if (!basis) return null;
  if (pack) {
    const derived = derivedBasis(pack);
    if (Number(basis.amount) === derived.amount && basis.unit === derived.unit) {
      if (pack.servings) return 'per serving';
      if (pack.quantity && dimensionOf(pack.quantity.unit) === 'count') return 'per item';
    }
  }
  return `per ${basis.amount} ${displayUnit(basis.unit)}`;
}

export function NutritionPanel({
  nutrition,
  pack,
  title = 'Nutrition',
  showBasis = true,
}: {
  nutrition: Nutrition | undefined;
  pack?: PackContext;
  title?: string;
  showBasis?: boolean;
}) {
  if (isNutritionUnknown(nutrition)) {
    return (
      <Stack spacing={1}>
        <Typography variant="h3">{title}</Typography>
        <Typography variant="body2" color="text.secondary">
          Not recorded yet.
        </Typography>
      </Stack>
    );
  }

  return (
    <Stack spacing={2.5}>
      <Stack direction="row" spacing={1} sx={{ alignItems: 'baseline' }}>
        <Typography variant="h3">{title}</Typography>
        {showBasis ? (
          <Typography variant="caption" color="text.secondary">
            {basisLabel(nutrition, pack)}
          </Typography>
        ) : null}
      </Stack>

      <Box>
        <Typography
          className="numeral"
          sx={{ fontSize: '2.75rem', fontWeight: 600, lineHeight: 1, letterSpacing: '-0.02em' }}
        >
          <MaybeNumber value={nutrition?.energy_kcal} fractionDigits={0} />
        </Typography>
        <Typography variant="caption" color="text.secondary">
          calories
        </Typography>
      </Box>

      <Stack direction="row" spacing={1}>
        {MACROS.map(({ key, label }) => (
          <Box
            key={key}
            sx={{
              flex: 1,
              px: 1.5,
              py: 1.25,
              borderRadius: 2,
              backgroundColor: 'action.hover',
            }}
          >
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
              {label}
            </Typography>
            <Typography className="numeral" sx={{ fontWeight: 600 }}>
              <MaybeNumber value={nutrition?.[key]} suffix="g" />
            </Typography>
          </Box>
        ))}
      </Stack>

      <Box
        component="dl"
        sx={{
          m: 0,
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) auto',
          rowGap: 0.75,
          columnGap: 3,
        }}
      >
        {REST.map(({ key, label, unit, indent }) => (
          <Box key={key} sx={{ display: 'contents' }}>
            <Box
              component="dt"
              sx={{
                color: 'text.secondary',
                fontSize: 14,
                pl: indent ? 2 : 0,
                pt: 0.75,
                borderTop: '1px solid',
                borderColor: 'divider',
              }}
            >
              {label}
            </Box>
            <Box
              component="dd"
              className="numeral"
              sx={{
                m: 0,
                fontWeight: 500,
                textAlign: 'right',
                pt: 0.75,
                borderTop: '1px solid',
                borderColor: 'divider',
              }}
            >
              <MaybeNumber value={nutrition?.[key]} suffix={unit} />
            </Box>
          </Box>
        ))}
      </Box>
    </Stack>
  );
}
