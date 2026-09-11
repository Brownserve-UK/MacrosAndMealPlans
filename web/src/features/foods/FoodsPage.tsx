import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import MenuItem from '@mui/material/MenuItem';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';
import TextField from '@mui/material/TextField';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { components } from '../../api/schema';
import type { IngredientListParams, PreparedMealListParams } from '../../api/queries';
import { useIngredients, usePreparedMeals } from '../../api/queries';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewIngredientDialog } from '../ingredients/NewIngredientDialog';
import { NewPreparedMealDialog } from '../prepared-meals/NewPreparedMealDialog';

type IngredientItem = components['schemas']['IngredientListItemDto'];
type PreparedMealItem = components['schemas']['PreparedMealListItemDto'];

type Tab = 'ingredients' | 'prepared_meals';
type Filter = 'all' | 'needs_products' | 'archived';
type SortKey = 'name' | 'created' | 'product_count';

type SortOption = {
  value: SortKey;
  label: string;
  sort_by: IngredientListParams['sort_by'];
  sort: IngredientListParams['sort'];
};

const SORTS = [
  { value: 'name', label: 'A-Z', sort_by: 'name', sort: 'asc' },
  { value: 'created', label: 'Date added', sort_by: 'created', sort: 'desc' },
  { value: 'product_count', label: 'Product count', sort_by: 'product_count', sort: 'desc' },
] as const satisfies readonly SortOption[];

const PER_PAGE = 25;

function describe(count: number): string {
  if (count === 0) return 'No products';
  return count === 1 ? '1 product' : `${count} products`;
}

export function FoodsPage() {
  const [tab, setTab] = useState<Tab>('ingredients');
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [sort, setSort] = useState<SortKey>('name');
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);

  const debounced = useDebounced(search, 300);
  const chosen = SORTS.find((option) => option.value === sort) ?? SORTS[0];

  const ingredientQuery = useIngredients({
    q: debounced || undefined,
    needs_products: filter === 'needs_products' || undefined,
    include_archived: filter === 'archived' || undefined,
    sort_by: chosen.sort_by,
    sort: chosen.sort,
    page,
    per_page: PER_PAGE,
  } satisfies IngredientListParams);

  const preparedMealQuery = usePreparedMeals({
    q: debounced || undefined,
    needs_products: filter === 'needs_products' || undefined,
    include_archived: filter === 'archived' || undefined,
    sort_by: chosen.sort_by,
    sort: chosen.sort,
    page,
    per_page: PER_PAGE,
  } satisfies PreparedMealListParams);

  const query = tab === 'ingredients' ? ingredientQuery : preparedMealQuery;
  const items: (IngredientItem | PreparedMealItem)[] = query.data?.items ?? [];
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE));

  function changeTab(next: Tab) {
    setTab(next);
    setPage(1);
  }

  return (
    <>
      <PageHeader
        title="Foods"
        subtitle="Generic foods that recipes ask for, and meals you plan without picking a brand."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            {tab === 'ingredients' ? 'Add ingredient' : 'Add prepared meal'}
          </Button>
        }
        search={{
          value: search,
          onChange: (next) => {
            setSearch(next);
            setPage(1);
          },
          placeholder: tab === 'ingredients' ? 'Search ingredients' : 'Search prepared meals',
        }}
      />

      <Tabs value={tab} onChange={(_, next: Tab) => changeTab(next)} sx={{ mb: 2.5 }}>
        <Tab value="ingredients" label="Ingredients" />
        <Tab value="prepared_meals" label="Prepared meals" />
      </Tabs>

      <Stack
        direction="row"
        spacing={1}
        sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1, alignItems: 'center' }}
      >
        {(
          [
            ['all', 'All'],
            ['needs_products', 'No products'],
            ['archived', 'Archived'],
          ] as const
        ).map(([value, label]) => (
          <Chip
            key={value}
            label={label}
            onClick={() => {
              setFilter(value);
              setPage(1);
            }}
            variant={filter === value ? 'filled' : 'outlined'}
            color={filter === value ? 'primary' : 'default'}
          />
        ))}

        <TextField
          select
          size="small"
          label="Sort"
          value={sort}
          onChange={(event) => {
            setSort(event.target.value as SortKey);
            setPage(1);
          }}
          sx={{ ml: 'auto', minWidth: 168 }}
        >
          {SORTS.map((option) => (
            <MenuItem key={option.value} value={option.value}>
              {option.label}
            </MenuItem>
          ))}
        </TextField>
      </Stack>

      {query.isError ? (
        <ErrorState error={query.error} onRetry={() => query.refetch()} />
      ) : query.isLoading ? (
        <Loading label={tab === 'ingredients' ? 'Finding ingredients' : 'Finding prepared meals'} />
      ) : items.length === 0 ? (
        <EmptyState
          title={search ? 'Nothing matched' : tab === 'ingredients' ? 'No ingredients yet' : 'No prepared meals yet'}
          description={
            search
              ? `Nothing matches "${search}".`
              : tab === 'ingredients'
                ? 'Add the generic foods your recipes use, like whole milk or basmati rice.'
                : 'Add a generic convenience meal, like a frozen lasagne, so it can be planned without a brand.'
          }
          action={
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
              {tab === 'ingredients' ? 'Add ingredient' : 'Add prepared meal'}
            </Button>
          }
        />
      ) : (
        <>
          <RecordListShell>
            {items.map((item) => (
              <Link
                key={item.id}
                to={tab === 'ingredients' ? '/ingredients/$id' : '/prepared-meals/$id'}
                params={{ id: item.id }}
              >
                <RecordRow
                  name={item.name}
                  detail={describe(item.mapped_product_count)}
                  muted={Boolean(item.archived_at)}
                  trailing={
                    item.mapped_product_count === 0 ? (
                      <Chip size="small" variant="outlined" label="No products" />
                    ) : null
                  }
                />
              </Link>
            ))}
          </RecordListShell>

          {pageCount > 1 ? (
            <Stack sx={{ alignItems: 'center', mt: 3 }}>
              <Pagination
                count={pageCount}
                page={page}
                onChange={(_, next) => {
                  setPage(next);
                  window.scrollTo({ top: 0 });
                }}
                shape="rounded"
                color="primary"
              />
            </Stack>
          ) : null}
        </>
      )}

      <NewIngredientDialog open={addOpen && tab === 'ingredients'} onClose={() => setAddOpen(false)} />
      <NewPreparedMealDialog open={addOpen && tab === 'prepared_meals'} onClose={() => setAddOpen(false)} />
    </>
  );
}
