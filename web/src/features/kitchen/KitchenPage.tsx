import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { PlannerMeal, StockItem } from '../../api/client';
import { useHouseholdPlannerWeek, useStock } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { ThingRow } from '../../components/ThingRow';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { CookDialog } from '../meal-plan/CookDialog';
import {
  CookedSection,
  type CookedFoodToPutAway,
} from '../meal-plan/CookedSection';
import { CookSomethingDialog } from '../meal-plan/CookSomethingDialog';
import { startOfWeekIso, todayIso } from '../meal-plan/date';
import { labelForSlot } from '../meal-plan/slots';
import { MoveCookedDialog } from '../stock/MoveCookedDialog';

type PlannedCook = {
  meal: PlannerMeal;
  componentId: string;
  name: string;
};

function servingsOf(item: StockItem): number {
  return 'quantity' in item.level ? item.level.quantity.amount : 0;
}

export function portionsToPutAway(items: StockItem[]): CookedFoodToPutAway[] {
  const grouped = new Map<string, CookedFoodToPutAway>();
  for (const item of items) {
    const servings = servingsOf(item);
    if (
      item.storage_location !== 'ambient' ||
      !item.prepared_recipe_id ||
      !item.prepared_batch_id ||
      servings <= 0
    ) {
      continue;
    }
    const current = grouped.get(item.prepared_recipe_id);
    if (current) {
      current.servings += servings;
    } else {
      grouped.set(item.prepared_recipe_id, {
        recipeId: item.prepared_recipe_id,
        name: item.prepared_batch_name ?? 'Cooked food',
        servings,
      });
    }
  }
  return [...grouped.values()].sort((a, b) => a.name.localeCompare(b.name));
}

export function KitchenPage() {
  const timeZone = useHouseholdTimeZone();
  const today = todayIso(timeZone);
  const week = useHouseholdPlannerWeek(startOfWeekIso(today));
  const stock = useStock({ per_page: 200 });
  const [cooking, setCooking] = useState<PlannedCook | null>(null);
  const [moving, setMoving] = useState<CookedFoodToPutAway | null>(null);
  const [cookingSomething, setCookingSomething] = useState(false);

  const planned = useMemo<PlannedCook[]>(
    () =>
      (week.data?.meals ?? [])
        .filter((meal) => meal.planned_on === today)
        .flatMap((meal) =>
          meal.foods
            .filter(
              (food) => food.item_kind === 'recipe' && food.needs_cooking && !food.cooked,
            )
            .map((food) => ({
              meal,
              componentId: food.id,
              name: food.item_name,
            })),
        )
        .sort((a, b) =>
          (a.meal.planned_time ?? '99:99').localeCompare(b.meal.planned_time ?? '99:99'),
        ),
    [today, week.data],
  );
  const leftovers = useMemo(
    () => portionsToPutAway(stock.data?.items ?? []),
    [stock.data],
  );

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;
  if (stock.isError) return <ErrorState error={stock.error} onRetry={() => stock.refetch()} />;

  return (
    <>
      <PageHeader title="Kitchen" />

      {week.isLoading || stock.isLoading ? (
        <Loading label="Loading kitchen" />
      ) : (
        <Stack spacing={3}>
          <Stack component="section" spacing={1} aria-labelledby="kitchen-today">
            <Typography variant="h3" id="kitchen-today">
              Today
            </Typography>
            {planned.length > 0 ? (
              <Stack spacing={1.5}>
                {planned.map((item) => {
                  const caption = [item.meal.planned_time, labelForSlot(item.meal.slot)]
                    .filter(Boolean)
                    .join(' · ');
                  return (
                    <ThingRow
                      key={`${item.meal.id}-${item.componentId}`}
                      concept="cook"
                      tone="secondary"
                      title={item.name}
                      caption={<span className="numeral">{caption}</span>}
                      action={
                        <Button variant="contained" size="small" onClick={() => setCooking(item)}>
                          Cooked it
                        </Button>
                      }
                    />
                  );
                })}
              </Stack>
            ) : (
              <Typography variant="body2" color="text.secondary">
                Nothing left to cook today.
              </Typography>
            )}
          </Stack>

          <CookedSection items={leftovers} onPutAway={setMoving} />

          <Button
            startIcon={<AddIcon />}
            onClick={() => setCookingSomething(true)}
            sx={{ alignSelf: 'flex-start' }}
          >
            Cooked something
          </Button>
        </Stack>
      )}

      {cooking ? (
        <CookDialog
          meal={cooking.meal}
          componentId={cooking.componentId}
          onClose={() => setCooking(null)}
        />
      ) : null}
      {moving ? (
        <MoveCookedDialog
          recipeId={moving.recipeId}
          name={moving.name}
          from="ambient"
          to="chilled"
          available={moving.servings}
          onClose={() => setMoving(null)}
        />
      ) : null}
      {cookingSomething ? (
        <CookSomethingDialog onClose={() => setCookingSomething(false)} />
      ) : null}
    </>
  );
}
