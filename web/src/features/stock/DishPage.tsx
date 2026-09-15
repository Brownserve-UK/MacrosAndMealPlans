import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { MealSlot, StockItem } from '../../api/client';
import { useRecipe, useRecipeNutrition, useStock, useStockAvailability, useStockEventsFor } from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { IconTile } from '../../components/IconTile';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { MoveCookedDialog } from './MoveCookedDialog';
import { MealEditorDialog } from '../meal-plan/MealEditorDialog';
import { MealSlotMenu } from '../meal-plan/MealSlotMenu';
import { todayIso } from '../meal-plan/date';
import { MAIN_SLOTS } from '../meal-plan/slots';

const PLACE_LABEL: Record<StockItem['storage_location'], string> = {
  ambient: 'Cupboard',
  chilled: 'Fridge',
  frozen: 'Freezer',
};

const PLACE_ORDER: StockItem['storage_location'][] = ['chilled', 'frozen', 'ambient'];

type Place = {
  location: StockItem['storage_location'];
  servings: number;
  useBy: string | null;
};

type Move = {
  from: StockItem['storage_location'];
  to: StockItem['storage_location'];
  available: number;
};

function show(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

function servingsOf(item: StockItem): number {
  return 'quantity' in item.level ? item.level.quantity.amount : 0;
}

function dateLabel(date: string) {
  return new Date(`${date}T00:00:00`).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  });
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

export function DishPage({ recipeId }: { recipeId: string }) {
  const recipe = useRecipe(recipeId);
  const nutrition = useRecipeNutrition(recipeId);
  const stock = useStock({ per_page: 200 });
  const availability = useStockAvailability();
  const [planning, setPlanning] = useState<MealSlot | null>(null);
  const [moving, setMoving] = useState<Move | null>(null);
  const timeZone = useHouseholdTimeZone();

  const portions = (stock.data?.items ?? []).filter(
    (item) => item.prepared_recipe_id === recipeId && servingsOf(item) > 0,
  );
  const events = useStockEventsFor(portions.map((item) => item.id));

  if (recipe.isError) return <ErrorState error={recipe.error} onRetry={() => recipe.refetch()} />;
  if (!recipe.data) return <Loading label="Loading this food" />;

  const byPlace = new Map<string, Place>();
  for (const item of portions) {
    const useBy = item.usability_deadline?.date ?? null;
    const found = byPlace.get(item.storage_location);
    if (found) {
      found.servings += servingsOf(item);
      if (useBy && (!found.useBy || useBy < found.useBy)) found.useBy = useBy;
    } else {
      byPlace.set(item.storage_location, {
        location: item.storage_location,
        servings: servingsOf(item),
        useBy,
      });
    }
  }
  const places = PLACE_ORDER.map((location) => byPlace.get(location)).filter(
    (place): place is Place => place !== undefined,
  );
  const left = places.reduce((total, place) => total + place.servings, 0);

  const cooked = (availability.data?.cooked_food ?? []).find((row) => row.recipe_id === recipeId);
  const spokenFor =
    cooked && cooked.availability.state === 'quantified'
      ? cooked.availability.planned_demand.amount
      : 0;

  const perServing = nutrition.data?.nutrition;

  return (
    <Box>
      <Link to="/stock" className="app-link">
        <BackLabel>Stock</BackLabel>
      </Link>

      <Paper sx={{ p: 3, mb: 2 }}>
        <Stack spacing={2.5}>
          <Stack direction="row" spacing={2} sx={{ alignItems: 'flex-start' }}>
            <IconTile
              concept={places[0]?.location === 'frozen' ? 'freezer' : 'fridge'}
              tone="primary"
              size={40}
            />
            <Box sx={{ flexGrow: 1, minWidth: 0 }}>
              <Typography variant="h2">{recipe.data.name}</Typography>
              <Typography variant="body2" color="text.secondary">
                {left === 0
                  ? 'None left'
                  : spokenFor > 0
                    ? `${show(spokenFor)} of ${show(left)} spoken for by planned meals`
                    : 'Not spoken for yet'}
              </Typography>
            </Box>
            <Chip size="small" label="Cooked food" variant="outlined" />
          </Stack>

          {left > 0 ? (
            <Stack spacing={1.25}>
              {places.map((place) => {
                const destination = place.location === 'frozen' ? 'chilled' : 'frozen';
                return (
                  <Stack
                    key={place.location}
                    direction="row"
                    spacing={2}
                    sx={{ alignItems: 'baseline', justifyContent: 'space-between' }}
                  >
                    <Typography variant="subtitle1">{PLACE_LABEL[place.location]}</Typography>
                    <Typography variant="caption" color="text.secondary" sx={{ flexGrow: 1 }}>
                      {place.useBy ? `use by ${dateLabel(place.useBy)}` : 'no date'}
                    </Typography>
                    <Button
                      size="small"
                      onClick={() => setMoving({ from: place.location, to: destination, available: place.servings })}
                    >
                      {destination === 'frozen' ? 'Freeze some' : 'Get some out'}
                    </Button>
                    <Typography variant="subtitle1" className="numeral" sx={{ fontWeight: 600 }}>
                      {place.servings === 1 ? '1 serving' : `${show(place.servings)} servings`}
                    </Typography>
                  </Stack>
                );
              })}
            </Stack>
          ) : (
            <Typography variant="body2" color="text.secondary">
              This has all been eaten.
            </Typography>
          )}

          {left > 0 ? (
            <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1 }}>
              <MealSlotMenu choices={MAIN_SLOTS} label="Plan it" onSelect={setPlanning} />
            </Stack>
          ) : null}
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
            {([
              ['Energy', perServing?.energy_kcal, 'kcal'],
              ['Protein', perServing?.protein_g, 'g'],
              ['Carbs', perServing?.carbohydrate_g, 'g'],
              ['Fat', perServing?.fat_g, 'g'],
            ] as const).map(([label, value, unit]) => (
              <Stack key={label} direction="row" sx={{ justifyContent: 'space-between' }}>
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
            Servings left
          </Typography>
          <Typography variant="h1" className="numeral" sx={{ mt: 1 }}>
            {show(left)}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
            {places.length > 1
              ? 'Pooled across everywhere it is kept, oldest eaten first.'
              : 'Oldest cook is eaten first.'}
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
        <Stack spacing={1.5} sx={{ mt: 1.5 }} divider={<Divider flexItem />}>
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
            <EmptyState title="Nothing recorded yet" description="Cooking and eating will show up here." />
          ) : null}
        </Stack>
      </Paper>

      {moving ? (
        <MoveCookedDialog
          recipeId={recipeId}
          name={recipe.data.name}
          from={moving.from}
          to={moving.to}
          available={moving.available}
          onClose={() => setMoving(null)}
        />
      ) : null}

      {planning ? (
        <MealEditorDialog
          open
          mode="household"
          onClose={() => setPlanning(null)}
          date={todayIso(timeZone)}
          slot={planning}
          meal={null}
          startWith={{ recipeId, name: recipe.data.name, servings: left }}
        />
      ) : null}
    </Box>
  );
}
