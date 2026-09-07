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

export type PlannableDish = { preparedBatchId: string; name: string; servings: number };

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

  const pressing = (stock.data?.items ?? [])
    .filter((item) => item.subject_kind === 'prepared_portion')
    .filter((item) => item.usability_deadline && item.usability_deadline.date <= horizon)
    .sort((a, b) =>
      (a.usability_deadline?.date ?? '').localeCompare(b.usability_deadline?.date ?? ''),
    );

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
                const servings = servingsOf(item.level);
                const amount = servings == null
                  ? ''
                  : `, ${servings === 1 ? '1 serving' : `${servings} servings`}`;
                const when = item.usability_deadline
                  ? `, ${deadlineLabel(item.usability_deadline.date, today)}`
                  : '';
                return `${item.prepared_batch_name ?? 'Cooked food'}${amount}${when}`;
              })
              .join(' · ')}
          </Typography>
        </Stack>

        {onPlan && soonest?.prepared_batch_id ? (
          <MealSlotMenu
            choices={MAIN_SLOTS}
            label="Plan it"
            variant="text"
            onSelect={(slot) =>
              onPlan(
                {
                  preparedBatchId: soonest.prepared_batch_id!,
                  name: soonest.prepared_batch_name ?? 'Cooked food',
                  servings: servingsOf(soonest.level) ?? 1,
                },
                slot,
              )
            }
          />
        ) : null}
      </Stack>
    </Paper>
  );
}
