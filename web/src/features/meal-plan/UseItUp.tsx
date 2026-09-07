import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { alpha } from '@mui/material/styles';
import type { MealSlot } from '../../api/client';
import { useStock } from '../../api/queries';
import type { components } from '../../api/schema';
import { IconTile } from '../../components/IconTile';
import { addDays, parseIsoDate } from './date';
import { MealSlotMenu } from './MealSlotMenu';
import { MAIN_SLOTS } from './slots';

export type PlannableDish = { recipeId: string; name: string; servings: number };

type Pressing = PlannableDish & { useBy: string };

const HORIZON_DAYS = 4;

function deadlineLabel(date: string, today: string): string {
  if (date < today) return 'past its use by';
  if (date === today) return 'by today';
  if (date === addDays(today, 1)) return 'by tomorrow';
  return `by ${parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long' })}`;
}

function servingsOf(level: components['schemas']['StockLevelDto']): number | null {
  return 'quantity' in level ? level.quantity.amount : null;
}

export function UseItUp({
  today,
  onPlan,
}: {
  today: string;
  onPlan?: (dish: PlannableDish, slot: MealSlot) => void;
}) {
  const stock = useStock({ per_page: 200 });
  const horizon = addDays(today, HORIZON_DAYS);

  const byPlace = new Map<string, Pressing>();
  for (const item of stock.data?.items ?? []) {
    const recipeId = item.prepared_recipe_id;
    const useBy = item.usability_deadline?.date;
    if (!recipeId || !useBy || useBy > horizon) continue;
    const servings = servingsOf(item.level) ?? 0;
    if (servings <= 0) continue;
    const key = `${recipeId}:${item.storage_location}`;
    const found = byPlace.get(key);
    if (found) {
      found.servings += servings;
      if (useBy < found.useBy) found.useBy = useBy;
    } else {
      byPlace.set(key, {
        recipeId,
        name: item.prepared_batch_name ?? 'Cooked food',
        servings,
        useBy,
      });
    }
  }
  const pressing = [...byPlace.values()].sort((a, b) => a.useBy.localeCompare(b.useBy));

  if (pressing.length === 0) return null;
  const soonest = pressing[0];

  return (
    <Paper
      sx={(theme) => ({
        px: 2.25,
        py: 1.75,
        mb: 3,
        bgcolor: alpha(theme.palette.warning.main, 0.07),
        borderColor: alpha(theme.palette.warning.main, 0.25),
      })}
    >
      <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
        <IconTile concept="dish" tone="warning" />
        <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
          <Typography variant="subtitle2">Use it up</Typography>
          <Typography variant="caption" color="text.secondary" className="numeral">
            {pressing
              .slice(0, 3)
              .map((item) => {
                const amount = item.servings === 1 ? '1 serving' : `${item.servings} servings`;
                return `${item.name}, ${amount}, ${deadlineLabel(item.useBy, today)}`;
              })
              .join(' · ')}
          </Typography>
        </Stack>

        {onPlan && soonest ? (
          <MealSlotMenu
            choices={MAIN_SLOTS}
            label="Plan it"
            variant="text"
            onSelect={(slot) =>
              onPlan(
                { recipeId: soonest.recipeId, name: soonest.name, servings: soonest.servings },
                slot,
              )
            }
          />
        ) : null}
      </Stack>
    </Paper>
  );
}
