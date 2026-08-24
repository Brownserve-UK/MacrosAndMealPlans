import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { components } from '../../api/schema';
import { useMembers } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewMemberDialog } from './NewMemberDialog';

type Item = components['schemas']['HouseholdMemberDto'];

type Filter = 'all' | 'no_account' | 'archived';

const PER_PAGE = 25;

function describe(item: Item): string {
  return item.has_account ? 'Has an account' : 'No account';
}

export function HouseholdPage() {
  const { principal } = useAuth();
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);

  const canManage = principal?.permissions.includes('household:write') ?? false;

  const debounced = useDebounced(search, 300);
  const query = useMembers({
    q: debounced || undefined,
    with_account: filter === 'no_account' ? false : undefined,
    include_archived: filter === 'archived' || undefined,
    page,
    per_page: PER_PAGE,
  });

  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const items = query.data?.items ?? [];
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE));

  return (
    <>
      <PageHeader
        title="Household"
        subtitle="The people this installation plans and cooks for."
        actions={
          canManage ? (
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
              Add someone
            </Button>
          ) : undefined
        }
        search={{
          value: search,
          onChange: (next) => {
            setSearch(next);
            setPage(1);
          },
          placeholder: 'Search people',
        }}
      />

      <Stack direction="row" spacing={1} sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1 }}>
        {(
          [
            ['all', 'All'],
            ['no_account', 'No account'],
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
      </Stack>

      {query.isLoading ? (
        <Loading label="Finding people" />
      ) : items.length === 0 ? (
        <EmptyState
          title={search ? 'Nobody matched' : 'Nobody here yet'}
          description={
            search
              ? `Nothing matches "${search}".`
              : 'Add the people in your household. They only need an account if they sign in.'
          }
          action={
            canManage ? (
              <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
                Add someone
              </Button>
            ) : undefined
          }
        />
      ) : (
        <>
          <RecordListShell>
            {items.map((item) => (
              <Link key={item.id} to="/household/$id" params={{ id: item.id }}>
                <RecordRow
                  name={item.display_name}
                  detail={describe(item)}
                  muted={Boolean(item.archived_at)}
                  trailing={
                    item.has_account ? null : (
                      <Chip size="small" variant="outlined" label="No account" />
                    )
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

      <NewMemberDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
