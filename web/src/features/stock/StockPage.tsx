import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import { useMemo, useState } from 'react';
import type { ProductAvailability } from '../../api/client';
import { useProducts, useStock, useStockAvailability } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewStockDialog } from './NewStockDialog';
import { groupSortDate, StockCard, type StockGroup } from './StockCard';
import { levelFor } from './stockLevel';

type SortKey = 'level' | 'name' | 'useby';

const SORTS: { value: SortKey; label: string }[] = [
  { value: 'level', label: 'Stock level' },
  { value: 'name', label: 'Name' },
  { value: 'useby', label: 'Use-by' },
];

function sortGroups(groups: StockGroup[], key: SortKey): StockGroup[] {
  const byName = (a: StockGroup, b: StockGroup) => a.productName.localeCompare(b.productName);
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

  const availabilityByProduct = useMemo(() => {
    const map = new Map<string, ProductAvailability>();
    for (const row of availability.data?.products ?? []) map.set(row.product_id, row);
    return map;
  }, [availability.data]);

  const groups = useMemo<StockGroup[]>(() => {
    const byProduct = new Map<string, StockGroup>();
    for (const item of stock.data?.items ?? []) {
      let group = byProduct.get(item.product_id);
      if (!group) {
        group = {
          productId: item.product_id,
          productName: productName.get(item.product_id) ?? 'Unknown product',
          items: [],
          availability: availabilityByProduct.get(item.product_id)?.availability ?? null,
        };
        byProduct.set(item.product_id, group);
      }
      group.items.push(item);
    }
    return [...byProduct.values()];
  }, [stock.data, productName, availabilityByProduct]);

  const visible = useMemo(() => {
    const needle = debounced.trim().toLowerCase();
    const filtered = needle
      ? groups.filter((group) => group.productName.toLowerCase().includes(needle))
      : groups;
    return sortGroups(filtered, sort);
  }, [groups, debounced, sort]);

  if (stock.isLoading) return <Loading label="Loading stock" />;
  if (stock.isError) return <ErrorState error={stock.error} onRetry={() => stock.refetch()} />;

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

      {groups.length > 0 && (
        <Stack direction="row" spacing={1} sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1 }}>
          {SORTS.map((option) => (
            <Chip
              key={option.value}
              label={option.label}
              onClick={() => setSort(option.value)}
              variant={sort === option.value ? 'filled' : 'outlined'}
              color={sort === option.value ? 'primary' : 'default'}
            />
          ))}
        </Stack>
      )}

      {groups.length === 0 ? (
        <EmptyState
          title="No stock recorded"
          description="Add what you have so planned meals can tell you what's missing."
        />
      ) : visible.length === 0 ? (
        <EmptyState title="Nothing matched" description={`Nothing matches "${search}".`} />
      ) : (
        <RecordListShell>
          {visible.map((group) => (
            <StockCard key={group.productId} group={group} />
          ))}
        </RecordListShell>
      )}

      <NewStockDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
