import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError } from '../../api/client';
import { useCook, usePlacePortions, useStock, useStockEventsFor } from '../../api/queries';
import { Link } from '@tanstack/react-router';
import { BackLabel } from '../../components/BackLink';
import { IconTile } from '../../components/IconTile';
import { ErrorState, Loading } from '../../components/States';
import { MealEditorDialog } from '../meal-plan/MealEditorDialog';
import { MealSlotMenu } from '../meal-plan/MealSlotMenu';
import { todayIso } from '../meal-plan/date';
import { MAIN_SLOTS } from '../meal-plan/slots';
import type { MealSlot } from '../../api/client';

function show(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

function whenLabel(at: string) {
  const moment = new Date(at);
  return Number.isNaN(moment.getTime())
    ? ''
    : moment.toLocaleString('en-GB', {
        weekday: 'long',
        day: 'numeric',
        month: 'long',
        hour: '2-digit',
        minute: '2-digit',
      });
}

function eventLabel(kind: string): string {
  if (kind === 'added') return 'Cooked';
  if (kind === 'consumed') return 'Eaten';
  if (kind === 'moved') return 'Put away';
  if (kind === 'released') return 'Put back';
  if (kind === 'discarded') return 'Thrown out';
  if (kind === 'archived') return 'Cleared';
  return kind.replace(/_/g, ' ');
}

export function DishPage({ batchId }: { batchId: string }) {
  const cook = useCook(batchId);
  const stock = useStock({ per_page: 200 });
  const place = usePlacePortions();
  const [planning, setPlanning] = useState<MealSlot | null>(null);
  const [error, setError] = useState<string | null>(null);

  const portions = (stock.data?.items ?? []).filter(
    (item) => item.prepared_batch_id === batchId,
  );
  const events = useStockEventsFor(portions.map((item) => item.id));

  if (cook.isError) return <ErrorState error={cook.error} onRetry={() => cook.refetch()} />;
  if (!cook.data) return <Loading label="Loading this cook" />;

  const left = portions.reduce(
    (total, item) => total + ('quantity' in item.level ? item.level.quantity.amount : 0),
    0,
  );
  const frozen = portions.some((item) => item.storage_location === 'frozen');
  const chilled = portions.some((item) => item.storage_location === 'chilled');
  const where = frozen && chilled
    ? 'In the fridge and the freezer'
    : frozen
      ? 'In the freezer'
      : chilled
        ? 'In the fridge'
        : 'Out of the freezer';
  const deadline = portions
    .map((item) => item.usability_deadline?.date)
    .filter((date): date is string => Boolean(date))
    .sort()[0];
  const perServing = cook.data.nutrition;

  async function moveAll(to: 'chilled' | 'frozen') {
    setError(null);
    try {
      await place.mutateAsync({
        id: batchId,
        body: { placements: [{ storage_location: to, servings: left }] },
      });
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not move this.');
    }
  }

  return (
    <Box>
      <Link to="/stock" className="app-link">
        <BackLabel>Stock</BackLabel>
      </Link>

      <Paper sx={{ p: 3, mb: 2 }}>
        <Stack spacing={2.5}>
          <Stack direction="row" spacing={2} sx={{ alignItems: 'flex-start' }}>
            <IconTile concept={frozen ? 'freezer' : 'fridge'} tone="primary" size={40} />
            <Box sx={{ flexGrow: 1, minWidth: 0 }}>
              <Typography variant="h2">{cook.data.item_name}</Typography>
              <Typography variant="body2" color="text.secondary">{where}</Typography>
            </Box>
            <Chip size="small" label="Cooked food" variant="outlined" />
          </Stack>

          {error ? <Alert severity="error" onClose={() => setError(null)}>{error}</Alert> : null}

          <Stack direction="row" spacing={1.5} sx={{ alignItems: 'baseline' }}>
            <Typography variant="h1" className="numeral">{show(left)}</Typography>
            <Typography color="text.secondary">
              {left === 1 ? 'serving left' : 'servings left'}
            </Typography>
          </Stack>

          {left > 0 ? (
            <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1 }}>
              <MealSlotMenu choices={MAIN_SLOTS} label="Plan it" onSelect={setPlanning} />
              <Button
                disabled={place.isPending || (frozen && !chilled)}
                onClick={() => void moveAll('frozen')}
              >
                {chilled && frozen ? 'Freeze all of it' : 'Move to freezer'}
              </Button>
              <Button
                disabled={place.isPending || (chilled && !frozen)}
                onClick={() => void moveAll('chilled')}
              >
                {chilled && frozen ? 'Fridge all of it' : 'Move to fridge'}
              </Button>
            </Stack>
          ) : (
            <Typography variant="body2" color="text.secondary">This has all been eaten.</Typography>
          )}
        </Stack>
      </Paper>

      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ mb: 2 }}>
        <Paper sx={{ flex: 1, p: 2.5 }}>
          <Typography
            variant="caption"
            sx={{ fontWeight: 600, letterSpacing: '0.04em', textTransform: 'uppercase', color: 'text.secondary' }}
          >
            A serving
          </Typography>
          <Stack spacing={0.75} sx={{ mt: 1.25 }}>
            {[
              ['Energy', perServing.energy_kcal, 'kcal'],
              ['Protein', perServing.protein_g, 'g'],
              ['Carbs', perServing.carbohydrate_g, 'g'],
              ['Fat', perServing.fat_g, 'g'],
            ].map(([label, value, unit]) => (
              <Stack key={String(label)} direction="row" sx={{ justifyContent: 'space-between' }}>
                <Typography variant="body2">{label}</Typography>
                <Typography variant="body2" color="text.secondary" className="numeral">
                  {value == null ? 'Not known' : `${Math.round(Number(value))} ${unit}`}
                </Typography>
              </Stack>
            ))}
          </Stack>
        </Paper>

        <Paper sx={{ flex: 1, p: 2.5 }}>
          <Typography
            variant="caption"
            sx={{ fontWeight: 600, letterSpacing: '0.04em', textTransform: 'uppercase', color: 'text.secondary' }}
          >
            Keeps until
          </Typography>
          <Typography variant="h2" sx={{ mt: 1 }}>
            {deadline
              ? new Date(deadline).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })
              : 'No date'}
          </Typography>
          <Typography variant="body2" color="text.secondary" className="numeral" sx={{ mt: 0.5 }}>
            {`Cooked ${whenLabel(cook.data.prepared_at)}, ${show(cook.data.servings_produced)} made`}
          </Typography>
        </Paper>
      </Stack>

      <Paper sx={{ p: 2.5 }}>
        <Typography
          variant="caption"
          sx={{ fontWeight: 600, letterSpacing: '0.04em', textTransform: 'uppercase', color: 'text.secondary' }}
        >
          History
        </Typography>
        <Stack spacing={1.5} sx={{ mt: 1.5 }}>
          {[...(events.data ?? [])]
            .sort((a, b) => b.occurred_at.localeCompare(a.occurred_at))
            .map((event) => (
            <Stack key={event.id} direction="row" spacing={2} sx={{ alignItems: 'center' }}>
              <Box sx={{ flexGrow: 1, minWidth: 0 }}>
                <Typography variant="body2">{eventLabel(event.kind)}</Typography>
                <Typography variant="caption" color="text.secondary">
                  {whenLabel(event.occurred_at)}
                </Typography>
              </Box>
              {event.quantity_delta ? (
                <Typography variant="body2" className="numeral" sx={{ fontWeight: 600 }}>
                  {`${event.quantity_delta.amount > 0 ? '+' : ''}${show(event.quantity_delta.amount)}`}
                </Typography>
              ) : null}
            </Stack>
          ))}
          {(events.data ?? []).length === 0 ? (
            <Typography variant="body2" color="text.secondary">Nothing recorded yet.</Typography>
          ) : null}
        </Stack>
      </Paper>

      {planning ? (
        <MealEditorDialog
          open
          mode="household"
          onClose={() => setPlanning(null)}
          date={todayIso()}
          slot={planning}
          meal={null}
          startWith={{ preparedBatchId: batchId, name: cook.data.item_name, servings: left }}
        />
      ) : null}
    </Box>
  );
}
