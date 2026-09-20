import AddIcon from '@mui/icons-material/AddOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { Fragment, useMemo, useState } from 'react';
import type { IngredientAvailability, PreparedMealAvailability, ProductAvailability, StockItem } from '../../api/client';
import { useProducts, useStock, useStockAvailability } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { addDays, startOfWeekIso, todayIso } from '../meal-plan/date';
import { NewStockDialog } from './NewStockDialog';
import {
  groupSortDate,
  IngredientCard,
  LOCATION_ORDER,
  PLACE_ORDER,
  PreparedPortionCard,
  primaryLocation,
  StockCard,
  StorageGroupHeading,
  withLocationGroups,
  type CookedFoodRow,
  type StockGroup,
} from './StockCard';
import { levelFor } from './stockLevel';

type View = 'ingredients' | 'products' | 'prepared';
type SortKey = 'level' | 'name' | 'useby';
type WindowKey = 'week' | 'fortnight' | 'all';

const SORTS: { value: SortKey; label: string }[] = [
  { value: 'level', label: 'Stock level' },
  { value: 'name', label: 'Name' },
  { value: 'useby', label: 'Use-by' },
];

const WINDOWS: { value: WindowKey; label: string }[] = [
  { value: 'week', label: 'This week' },
  { value: 'fortnight', label: '14 days' },
  { value: 'all', label: 'All planned' },
];

const STRIP_HEADING: Record<WindowKey, string> = {
  week: 'Short this week',
  fortnight: 'Short in 14 days',
  all: 'Short',
};

function windowRange(windowKey: WindowKey): { from: string; to: string } {
  const from = todayIso();
  if (windowKey === 'fortnight') return { from, to: addDays(from, 13) };
  if (windowKey === 'all') return { from, to: addDays(from, 365) };
  return { from, to: addDays(startOfWeekIso(from), 6) };
}

function sortGroups(groups: StockGroup[], key: SortKey): StockGroup[] {
  const byName = (a: StockGroup, b: StockGroup) => a.name.localeCompare(b.name);
  const sorted = [...groups];
  if (key === 'name') return sorted.sort(byName);
  if (key === 'useby') {
    return sorted.sort((a, b) => {
      const da = groupSortDate(a);
      const db = groupSortDate(b);
      if (da && db) return da.localeCompare(db) || byName(a, b);
      if (da) return -1;
      if (db) return 1;
      return byName(a, b);
    });
  }
  return sorted.sort((a, b) => {
    const la = levelFor(a.availability);
    const lb = levelFor(b.availability);
    return (
      la.sortRank - lb.sortRank ||
      la.freeFraction - lb.freeFraction ||
      byName(a, b)
    );
  });
}

export function groupLocation(group: StockGroup): StockItem['storage_location'] {
  return primaryLocation(group.items.map((item) => item.storage_location));
}

function byLocation(groups: StockGroup[]): StockGroup[] {
  return [...groups].sort(
    (a, b) => LOCATION_ORDER.indexOf(groupLocation(a)) - LOCATION_ORDER.indexOf(groupLocation(b)),
  );
}

function sortCooked(rows: CookedFoodRow[], key: SortKey): CookedFoodRow[] {
  const byName = (a: CookedFoodRow, b: CookedFoodRow) => a.name.localeCompare(b.name);
  const sorted = [...rows];
  if (key === 'name') return sorted.sort(byName);
  if (key === 'level') return sorted.sort((a, b) => a.servings - b.servings || byName(a, b));
  return sorted.sort((a, b) => {
    if (a.useBy && b.useBy) return a.useBy.localeCompare(b.useBy) || byName(a, b);
    if (a.useBy) return -1;
    if (b.useBy) return 1;
    return byName(a, b);
  });
}

function ShortStrip({
  heading,
  items,
}: {
  heading: string;
  items: { id: string; name: string; amount: string }[];
}) {
  if (items.length === 0) return null;

  return (
    <Paper
      variant="outlined"
      sx={{ mb: 3, p: 2, borderLeft: '3px solid', borderLeftColor: 'warning.main' }}
    >
      <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 1 }}>
        <Typography variant="h3">{heading}</Typography>
        <Button component={Link} to="/shopping" size="small" sx={{ ml: 'auto' }}>
          Add to shopping
        </Button>
      </Stack>
      <Stack spacing={0.75}>
        {items.map((item) => (
          <Stack key={item.id} direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
            <Box
              aria-hidden
              sx={{ width: 6, height: 6, borderRadius: '50%', backgroundColor: 'warning.main', flexShrink: 0 }}
            />
            <Typography variant="body2" sx={{ flexGrow: 1, minWidth: 0 }} noWrap>
              {item.name}
            </Typography>
            <Typography
              variant="body2"
              className="numeral"
              sx={{ color: 'warning.main', fontWeight: 600, flexShrink: 0 }}
            >
              {item.amount}
            </Typography>
          </Stack>
        ))}
      </Stack>
    </Paper>
  );
}

export function StockPage() {
  const [addOpen, setAddOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [view, setView] = useState<View>('ingredients');
  const [sort, setSort] = useState<SortKey>('level');
  const [windowKey, setWindowKey] = useState<WindowKey>('week');
  const debounced = useDebounced(search, 200);
  const range = useMemo(() => windowRange(windowKey), [windowKey]);

  const stock = useStock({ per_page: 200 });
  const availability = useStockAvailability(undefined, range);
  const products = useProducts({ per_page: 200 });

  const productName = useMemo(() => {
    const map = new Map<string, string>();
    for (const product of products.data?.items ?? []) map.set(product.id, product.name);
    return map;
  }, [products.data]);

  const ingredientOfProduct = useMemo(() => {
    const map = new Map<string, string>();
    for (const product of products.data?.items ?? []) {
      if (product.mapped_ingredient_id) map.set(product.id, product.mapped_ingredient_id);
    }
    return map;
  }, [products.data]);

  const availabilityByProduct = useMemo(() => {
    const map = new Map<string, ProductAvailability>();
    for (const row of availability.data?.products ?? []) map.set(row.product_id, row);
    return map;
  }, [availability.data]);

  const availabilityByIngredient = useMemo(() => {
    const map = new Map<string, IngredientAvailability>();
    for (const row of availability.data?.ingredients ?? []) map.set(row.ingredient_id, row);
    return map;
  }, [availability.data]);

  const preparedMealOfProduct = useMemo(() => {
    const map = new Map<string, string>();
    for (const product of products.data?.items ?? []) {
      if (product.mapped_prepared_meal_id) map.set(product.id, product.mapped_prepared_meal_id);
    }
    return map;
  }, [products.data]);

  const availabilityByPreparedMeal = useMemo(() => {
    const map = new Map<string, PreparedMealAvailability>();
    for (const row of availability.data?.prepared_meals ?? []) map.set(row.prepared_meal_id, row);
    return map;
  }, [availability.data]);

  const productGroups = useMemo<StockGroup[]>(() => {
    const byProduct = new Map<string, StockGroup>();
    for (const item of stock.data?.items ?? []) {
      const productId = item.product_id;
      if (!productId) continue;
      let group = byProduct.get(productId);
      if (!group) {
        group = {
          id: productId,
          name: productName.get(productId) ?? 'Unknown product',
          items: [],
          availability: availabilityByProduct.get(productId)?.availability ?? null,
        };
        byProduct.set(productId, group);
      }
      group.items.push(item);
    }
    return [...byProduct.values()];
  }, [stock.data, productName, availabilityByProduct]);

  const preparedRows = useMemo<CookedFoodRow[]>(() => {
    type Place = { servings: number; useBy: string | null };
    const byRecipe = new Map<
      string,
      { name: string; total: number; soonest: string | null; places: Map<StockItem['storage_location'], Place> }
    >();
    for (const item of stock.data?.items ?? []) {
      const recipeId = item.prepared_recipe_id;
      if (!recipeId) continue;
      const servings = 'quantity' in item.level ? item.level.quantity.amount : 0;
      if (servings <= 0) continue;
      const useBy = item.usability_deadline?.date ?? null;
      let row = byRecipe.get(recipeId);
      if (!row) {
        row = {
          name: item.prepared_batch_name ?? 'Cooked food',
          total: 0,
          soonest: null,
          places: new Map(),
        };
        byRecipe.set(recipeId, row);
      }
      row.total += servings;
      if (useBy && (!row.soonest || useBy < row.soonest)) row.soonest = useBy;
      const place = row.places.get(item.storage_location);
      if (place) {
        place.servings += servings;
        if (useBy && (!place.useBy || useBy < place.useBy)) place.useBy = useBy;
      } else {
        row.places.set(item.storage_location, { servings, useBy });
      }
    }
    return [...byRecipe.entries()].map(([recipeId, row]) => ({
      key: recipeId,
      recipeId,
      name: row.name,
      servings: row.total,
      useBy: row.soonest,
      places: PLACE_ORDER.filter((location) => row.places.has(location)).map((location) => {
        const place = row.places.get(location) as Place;
        return { location, servings: place.servings, useBy: place.useBy };
      }),
    }));
  }, [stock.data]);

  const ingredientGroups = useMemo<{ group: StockGroup; productCount: number; kind: 'ingredient' }[]>(() => {
    const byIngredient = new Map<string, { group: StockGroup; products: Set<string> }>();
    for (const item of stock.data?.items ?? []) {
      if (!item.product_id) continue;
      const ingredientId = ingredientOfProduct.get(item.product_id);
      if (!ingredientId) continue;
      let entry = byIngredient.get(ingredientId);
      if (!entry) {
        entry = {
          group: {
            id: ingredientId,
            name: availabilityByIngredient.get(ingredientId)?.name ?? 'Unknown ingredient',
            items: [],
            availability: availabilityByIngredient.get(ingredientId)?.availability ?? null,
          },
          products: new Set(),
        };
        byIngredient.set(ingredientId, entry);
      }
      entry.group.items.push(item);
      entry.products.add(item.product_id);
    }
    return [...byIngredient.values()].map((entry) => ({
      group: entry.group,
      productCount: entry.products.size,
      kind: 'ingredient' as const,
    }));
  }, [stock.data, ingredientOfProduct, availabilityByIngredient]);

  const preparedMealGroups = useMemo<{ group: StockGroup; productCount: number; kind: 'prepared_meal' }[]>(() => {
    const byPreparedMeal = new Map<string, { group: StockGroup; products: Set<string> }>();
    for (const item of stock.data?.items ?? []) {
      if (!item.product_id) continue;
      const preparedMealId = preparedMealOfProduct.get(item.product_id);
      if (!preparedMealId) continue;
      let entry = byPreparedMeal.get(preparedMealId);
      if (!entry) {
        entry = {
          group: {
            id: preparedMealId,
            name: availabilityByPreparedMeal.get(preparedMealId)?.name ?? 'Unknown prepared meal',
            items: [],
            availability: availabilityByPreparedMeal.get(preparedMealId)?.availability ?? null,
          },
          products: new Set(),
        };
        byPreparedMeal.set(preparedMealId, entry);
      }
      entry.group.items.push(item);
      entry.products.add(item.product_id);
    }
    return [...byPreparedMeal.values()].map((entry) => ({
      group: entry.group,
      productCount: entry.products.size,
      kind: 'prepared_meal' as const,
    }));
  }, [stock.data, preparedMealOfProduct, availabilityByPreparedMeal]);

  const foodGroups = useMemo(() => [...ingredientGroups, ...preparedMealGroups], [ingredientGroups, preparedMealGroups]);

  const shortFoods = useMemo(() => {
    return foodGroups
      .map((entry) => ({ id: entry.group.id, name: entry.group.name, level: levelFor(entry.group.availability) }))
      .filter((entry) => entry.level.figure?.short)
      .sort((a, b) => a.level.freeFraction - b.level.freeFraction || a.name.localeCompare(b.name))
      .map((entry) => ({ id: entry.id, name: entry.name, amount: entry.level.figure?.shortAmount ?? '' }));
  }, [foodGroups]);

  const visibleProducts = useMemo(() => {
    const needle = debounced.trim().toLowerCase();
    const filtered = needle
      ? productGroups.filter((group) => group.name.toLowerCase().includes(needle))
      : productGroups;
    return byLocation(sortGroups(filtered, sort));
  }, [productGroups, debounced, sort]);

  const visiblePrepared = useMemo(() => {
    const needle = debounced.trim().toLowerCase();
    const filtered = needle
      ? preparedRows.filter((row) => row.name.toLowerCase().includes(needle))
      : preparedRows;
    return sortCooked(filtered, sort);
  }, [preparedRows, debounced, sort]);

  const visibleFoods = useMemo(() => {
    const needle = debounced.trim().toLowerCase();
    const filtered = needle
      ? foodGroups.filter((entry) => entry.group.name.toLowerCase().includes(needle))
      : foodGroups;
    const order = new Map(
      byLocation(
        sortGroups(
          filtered.map((entry) => entry.group),
          sort,
        ),
      ).map((group, index) => [group.id, index]),
    );
    return [...filtered].sort((a, b) => (order.get(a.group.id) ?? 0) - (order.get(b.group.id) ?? 0));
  }, [foodGroups, debounced, sort]);

  if (stock.isLoading) return <Loading label="Loading stock" />;
  if (stock.isError) return <ErrorState error={stock.error} onRetry={() => stock.refetch()} />;

  const empty = productGroups.length === 0 && preparedRows.length === 0;
  const showing =
    view === 'ingredients'
      ? visibleFoods.length
      : view === 'prepared'
        ? visiblePrepared.length
        : visibleProducts.length;

  return (
    <>
      <PageHeader
        title="Stock"
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            Add stock
          </Button>
        }
        search={{
          value: search,
          onChange: setSearch,
          placeholder: 'Search stock',
        }}
      />

      <ShortStrip heading={STRIP_HEADING[windowKey]} items={shortFoods} />

      {!empty && (
        <>
          <Tabs
            value={view}
            onChange={(_, next: View) => setView(next)}
            sx={{ mb: 2.5 }}
          >
            <Tab value="ingredients" label="Foods" />
            <Tab value="products" label="Products" />
            <Tab value="prepared" label="Cooked" />
          </Tabs>

          <Stack
            direction="row"
            spacing={1}
            sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1, alignItems: 'center' }}
          >
            <ToggleButtonGroup
              size="small"
              exclusive
              value={windowKey}
              onChange={(_event, next: WindowKey | null) => {
                if (next) setWindowKey(next);
              }}
              aria-label="Demand window"
            >
              {WINDOWS.map((option) => (
                <ToggleButton key={option.value} value={option.value}>
                  {option.label}
                </ToggleButton>
              ))}
            </ToggleButtonGroup>

            <TextField
              select
              size="small"
              value={sort}
              onChange={(event) => setSort(event.target.value as SortKey)}
              slotProps={{ select: { 'aria-label': 'Sort' } }}
              sx={{ ml: 'auto', minWidth: 168 }}
            >
              {SORTS.map((option) => (
                <MenuItem key={option.value} value={option.value}>
                  {option.label}
                </MenuItem>
              ))}
            </TextField>
          </Stack>
        </>
      )}

      {empty ? (
        <EmptyState
          title="No stock recorded"
          description="Add what you have so planned meals can tell you what's missing."
        />
      ) : showing === 0 ? (
        search.trim() ? (
          <EmptyState title="Nothing matched" description={`Nothing matches "${search}".`} />
        ) : (
          view === 'prepared' ? (
            <EmptyState
              title="Nothing cooked yet"
              description="Cook a recipe and any servings you don't eat will wait here."
            />
          ) : (
            <EmptyState
              title="Nothing mapped to a food"
              description="Map your products to a food, or switch to Products."
            />
          )
        )
      ) : view === 'ingredients' ? (
        <RecordListShell>
          {withLocationGroups(visibleFoods, (entry) => groupLocation(entry.group)).map(({ item: entry, heading }) => (
            <Fragment key={entry.group.id}>
              {heading ? <StorageGroupHeading label={heading.label} count={heading.count} /> : null}
              <IngredientCard group={entry.group} productCount={entry.productCount} kind={entry.kind} />
            </Fragment>
          ))}
        </RecordListShell>
      ) : view === 'prepared' ? (
        <RecordListShell>
          {visiblePrepared.map((row) => (
            <PreparedPortionCard key={row.key} row={row} />
          ))}
        </RecordListShell>
      ) : (
        <RecordListShell>
          {withLocationGroups(visibleProducts, groupLocation).map(({ item: group, heading }) => (
            <Fragment key={group.id}>
              {heading ? <StorageGroupHeading label={heading.label} count={heading.count} /> : null}
              <StockCard group={group} />
            </Fragment>
          ))}
        </RecordListShell>
      )}

      <NewStockDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
