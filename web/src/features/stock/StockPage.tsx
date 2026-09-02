import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';
import TextField from '@mui/material/TextField';
import { useMemo, useState } from 'react';
import type { IngredientAvailability, ProductAvailability } from '../../api/client';
import { useProducts, useStock, useStockAvailability } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewStockDialog } from './NewStockDialog';
import { groupSortDate, IngredientCard, StockCard, type StockGroup } from './StockCard';
import { levelFor } from './stockLevel';

type View = 'ingredients' | 'products';
type SortKey = 'level' | 'name' | 'useby';

const SORTS: { value: SortKey; label: string }[] = [
  { value: 'level', label: 'Stock level' },
  { value: 'name', label: 'Name' },
  { value: 'useby', label: 'Use-by' },
];

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

export function StockPage() {
  const [addOpen, setAddOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [view, setView] = useState<View>('ingredients');
  const [sort, setSort] = useState<SortKey>('level');
  const debounced = useDebounced(search, 200);

  const stock = useStock({ per_page: 200 });
  const availability = useStockAvailability();
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

  const productGroups = useMemo<StockGroup[]>(() => {
    const byProduct = new Map<string, StockGroup>();
    for (const item of stock.data?.items ?? []) {
      let group = byProduct.get(item.product_id);
      if (!group) {
        group = {
          id: item.product_id,
          name: productName.get(item.product_id) ?? 'Unknown product',
          items: [],
          availability: availabilityByProduct.get(item.product_id)?.availability ?? null,
        };
        byProduct.set(item.product_id, group);
      }
      group.items.push(item);
    }
    return [...byProduct.values()];
  }, [stock.data, productName, availabilityByProduct]);

  // Products we hold that aren't mapped to an ingredient have nowhere to sit here, so they only
  // ever appear under Products.
  const ingredientGroups = useMemo<{ group: StockGroup; productCount: number }[]>(() => {
    const byIngredient = new Map<string, { group: StockGroup; products: Set<string> }>();
    for (const item of stock.data?.items ?? []) {
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
    }));
  }, [stock.data, ingredientOfProduct, availabilityByIngredient]);

  const visibleProducts = useMemo(() => {
    const needle = debounced.trim().toLowerCase();
    const filtered = needle
      ? productGroups.filter((group) => group.name.toLowerCase().includes(needle))
      : productGroups;
    return sortGroups(filtered, sort);
  }, [productGroups, debounced, sort]);

  const visibleIngredients = useMemo(() => {
    const needle = debounced.trim().toLowerCase();
    const filtered = needle
      ? ingredientGroups.filter((entry) => entry.group.name.toLowerCase().includes(needle))
      : ingredientGroups;
    const order = new Map(
      sortGroups(
        filtered.map((entry) => entry.group),
        sort,
      ).map((group, index) => [group.id, index]),
    );
    return [...filtered].sort((a, b) => (order.get(a.group.id) ?? 0) - (order.get(b.group.id) ?? 0));
  }, [ingredientGroups, debounced, sort]);

  if (stock.isLoading) return <Loading label="Loading stock" />;
  if (stock.isError) return <ErrorState error={stock.error} onRetry={() => stock.refetch()} />;

  const empty = productGroups.length === 0;
  const showing = view === 'ingredients' ? visibleIngredients.length : visibleProducts.length;

  return (
    <>
      <PageHeader
        title="Stock"
        subtitle="What's in the house, and how much is still free after planned meals."
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

      {!empty && (
        <>
          <Tabs
            value={view}
            onChange={(_, next: View) => setView(next)}
            sx={{ mb: 2.5 }}
          >
            <Tab value="ingredients" label="Ingredients" />
            <Tab value="products" label="Products" />
          </Tabs>

          <Stack
            direction="row"
            spacing={1}
            sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1, alignItems: 'center' }}
          >
            <TextField
              select
              size="small"
              label="Sort"
              value={sort}
              onChange={(event) => setSort(event.target.value as SortKey)}
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
          <EmptyState
            title="Nothing mapped to an ingredient"
            description="Map your products to ingredients, or switch to Products."
          />
        )
      ) : view === 'ingredients' ? (
        <RecordListShell>
          {visibleIngredients.map((entry) => (
            <IngredientCard
              key={entry.group.id}
              group={entry.group}
              productCount={entry.productCount}
            />
          ))}
        </RecordListShell>
      ) : (
        <RecordListShell>
          {visibleProducts.map((group) => (
            <StockCard key={group.id} group={group} />
          ))}
        </RecordListShell>
      )}

      <NewStockDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
