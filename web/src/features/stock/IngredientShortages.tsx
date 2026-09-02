import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { IngredientAvailability } from '../../api/client';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { RecordListShell } from '../../components/RecordList';
import { gapLabel } from './demandGap';
import { levelFor } from './stockLevel';

export type IngredientRow = {
  ingredientId: string;
  name: string;
  row: IngredientAvailability;
};

export function IngredientShortages({ rows }: { rows: IngredientRow[] }) {
  if (rows.length === 0) return null;

  return (
    <Stack spacing={1.25} sx={{ mt: 3 }}>
      <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
        Ingredients to watch
      </Typography>
      <RecordListShell>
        {rows.map(({ ingredientId, name, row }) => {
          const level = levelFor(row.availability);
          const gap = gapLabel(row.demand_gaps);
          return (
            <Box
              key={ingredientId}
              data-testid={`ingredient-shortage-${ingredientId}`}
              sx={{
                display: 'flex',
                alignItems: 'center',
                gap: 1.75,
                px: { xs: 2, sm: 2.5 },
                py: 1.5,
              }}
            >
              <InitialsAvatar name={name} size={40} />
              <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }} noWrap>
                  {name}
                </Typography>
                {gap ? (
                  <Chip
                    size="small"
                    color="warning"
                    variant="outlined"
                    label={gap}
                    sx={{ alignSelf: 'flex-start', mt: 0.25 }}
                  />
                ) : (
                  level.detailLine && (
                    <Typography variant="caption" color="text.secondary" noWrap>
                      {level.detailLine}
                    </Typography>
                  )
                )}
              </Stack>
              {level.figure && (
                <Typography variant="body2" sx={{ color: level.colour, fontWeight: 600 }}>
                  {level.figure.needed} / {level.figure.available}
                </Typography>
              )}
            </Box>
          );
        })}
      </RecordListShell>
    </Stack>
  );
}
