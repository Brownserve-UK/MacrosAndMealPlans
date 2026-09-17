import AddIcon from '@mui/icons-material/AddOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { MealPlanComponent, StockItem } from '../../api/client';
import { usePlannerWeek, useStock } from '../../api/queries';
import { KindChip } from '../../components/KindChip';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { ThingRow } from '../../components/ThingRow';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { CookDialog, type PlannedCook } from '../meal-plan/CookDialog';
import {
  CookedSection,
  type CookedFoodToPutAway,
} from '../meal-plan/CookedSection';
import { CookSomethingDialog } from '../meal-plan/CookSomethingDialog';
import { parseIsoDate, startOfWeekIso, todayIso } from '../meal-plan/date';
import { formatMinutes, SLOT_ORDER } from '../meal-plan/planner/plannerWeek';
import type { GroupView, OccasionView, PlannerWeek } from '../meal-plan/planner/types';
import { labelForSlot } from '../meal-plan/slots';
import { MoveCookedDialog } from '../stock/MoveCookedDialog';

export type Cookable = {
  occasion: OccasionView;
  group: GroupView;
  kind: 'recipe' | 'product';
  recipe: MealPlanComponent | null;
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

export function cookables(week: PlannerWeek | undefined): Cookable[] {
  const out: Cookable[] = [];
  for (const day of week?.days ?? []) {
    for (const occasion of day.occasions) {
      if (!occasion) continue;
      for (const group of occasion.groups) {
        if (group.ad_hoc) continue;
        const first = group.components[0];
        if (!first) continue;
        if (first.item_kind === 'recipe') {
          const recipe =
            group.components.find(
              (component) => component.item_kind === 'recipe' && component.needs_cooking && !component.cooked,
            ) ?? null;
          if (!recipe) continue;
          out.push({ occasion, group, kind: 'recipe', recipe });
        } else if (first.item_kind === 'product') {
          out.push({ occasion, group, kind: 'product', recipe: null });
        }
      }
    }
  }
  return out.sort((a, b) => {
    if (a.occasion.planned_on !== b.occasion.planned_on) {
      return a.occasion.planned_on.localeCompare(b.occasion.planned_on);
    }
    const byTime = (a.occasion.effective_time ?? '99:99').localeCompare(b.occasion.effective_time ?? '99:99');
    if (byTime !== 0) return byTime;
    return SLOT_ORDER.indexOf(a.occasion.slot) - SLOT_ORDER.indexOf(b.occasion.slot);
  });
}

function cookOf(item: Cookable): PlannedCook | null {
  const recipe = item.recipe;
  if (!recipe || recipe.item_kind !== 'recipe') return null;
  return {
    entryId: item.group.id,
    componentId: recipe.id,
    recipeId: recipe.recipe_id,
    name: item.group.name,
    planned: item.group.effective_cooking_servings,
  };
}

function DayLabel({ date }: { date: string }) {
  return (
    <Box
      aria-hidden
      sx={{
        width: 34,
        flexShrink: 0,
        textAlign: 'center',
        fontSize: '0.78rem',
        fontWeight: 600,
        color: 'text.secondary',
      }}
    >
      {parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'short' })}
    </Box>
  );
}

function Caption({ item, withTime }: { item: Cookable; withTime: boolean }) {
  const parts = [
    labelForSlot(item.occasion.slot),
    withTime ? item.occasion.effective_time : null,
    `cooking ${item.group.effective_cooking_servings}`,
    item.group.cook_minutes ? formatMinutes(item.group.cook_minutes) : null,
  ].filter((part): part is string => Boolean(part));
  return (
    <span className="numeral">
      {parts.join(' · ')}
      {item.group.to_buy > 0 ? (
        <Box component="span" sx={{ color: 'warning.main' }}>
          {` · ${item.group.to_buy} to buy`}
        </Box>
      ) : null}
    </span>
  );
}

export function KitchenPage() {
  const timeZone = useHouseholdTimeZone();
  const today = todayIso(timeZone);
  const week = usePlannerWeek(startOfWeekIso(today));
  const stock = useStock({ per_page: 200 });
  const [cooking, setCooking] = useState<PlannedCook | null>(null);
  const [moving, setMoving] = useState<CookedFoodToPutAway | null>(null);
  const [cookingSomething, setCookingSomething] = useState(false);

  const all = useMemo(() => cookables(week.data), [week.data]);
  const planned = all.filter((item) => item.occasion.planned_on === today);
  const comingUp = all.filter((item) => item.occasion.planned_on > today);
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
                  const cook = cookOf(item);
                  return (
                    <ThingRow
                      key={item.group.id}
                      concept="cook"
                      tone="secondary"
                      title={item.group.name}
                      caption={<Caption item={item} withTime />}
                      chip={<KindChip kind={item.kind} />}
                      action={
                        cook ? (
                          <Button variant="contained" size="small" onClick={() => setCooking(cook)}>
                            Cooked it
                          </Button>
                        ) : null
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

          <Stack component="section" spacing={1} aria-labelledby="kitchen-coming-up">
            <Typography variant="h3" id="kitchen-coming-up">
              Coming up
            </Typography>
            {comingUp.length > 0 ? (
              <Stack spacing={1.5}>
                {comingUp.map((item) => (
                  <ThingRow
                    key={item.group.id}
                    concept="cook"
                    leading={<DayLabel date={item.occasion.planned_on} />}
                    title={item.group.name}
                    caption={<Caption item={item} withTime={false} />}
                    chip={<KindChip kind={item.kind} />}
                  />
                ))}
              </Stack>
            ) : (
              <Typography variant="body2" color="text.secondary">
                Nothing else to cook this week.
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

      {cooking ? <CookDialog cook={cooking} onClose={() => setCooking(null)} /> : null}
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
