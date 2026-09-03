import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link, useNavigate } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type StockItem } from '../../api/client';
import {
  useArchiveStockItem,
  useProduct,
  useStockEvents,
  useStockItem,
  useUpdateStockItem,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { BackLabel } from '../../components/BackLink';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { displayUnit } from '../../components/UnitSelect';
import {
  draftToLevel,
  StockFields,
  validateStockDraft,
  type StockDraft,
} from './StockFields';

function draftFrom(item: StockItem): StockDraft {
  const level = item.level;
  return {
    product: null,
    trackingMode: level.mode,
    unit: level.mode === 'not_tracked' ? 'g' : level.quantity.unit,
    quantity: level.mode === 'not_tracked' ? '' : String(level.quantity.amount),
    storageLocation: item.storage_location,
    note: item.note ?? '',
  };
}

function History({ id }: { id: string }) {
  const events = useStockEvents(id);
  if (events.isLoading) return <Loading label="Loading history" />;
  if (events.isError) return <ErrorState error={events.error} />;
  const rows = events.data ?? [];
  if (rows.length === 0) return <Typography variant="body2">No history yet.</Typography>;
  return (
    <Stack spacing={1.25}>
      {rows.map((event) => (
        <Stack key={event.id} direction="row" spacing={2} sx={{ justifyContent: 'space-between' }}>
          <Typography variant="body2">
            {event.kind}
            {event.quantity_delta
              ? ` · ${event.quantity_delta.amount} ${displayUnit(event.quantity_delta.unit)}`
              : ''}
            {event.source_label ? ` · ${event.source_label}` : ''}
            {event.note ? ` · ${event.note}` : ''}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {new Date(event.occurred_at).toLocaleString()}
          </Typography>
        </Stack>
      ))}
    </Stack>
  );
}

export function StockItemPage({ id }: { id: string }) {
  const navigate = useNavigate();
  const { principal } = useAuth();
  const item = useStockItem(id);
  const update = useUpdateStockItem();
  const archive = useArchiveStockItem();

  const [draft, setDraft] = useState<StockDraft | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [showHistory, setShowHistory] = useState(false);

  const product = useProduct(item.data?.product_id ?? '', { enabled: Boolean(item.data) });

  if (item.isLoading) return <Loading label="Loading stock item" />;
  if (item.isError || !item.data) {
    return <ErrorState error={item.error} onRetry={() => item.refetch()} />;
  }

  const current = item.data;
  const working = draft ?? draftFrom(current);
  const canSeeHistory = principal?.permissions.includes('stock:history') ?? false;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validateStockDraft({ ...working, product: { id: current.product_id } as never });
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    try {
      await update.mutateAsync({
        id,
        revision: current.revision,
        body: {
          level: draftToLevel(working),
          storage_location: working.storageLocation,
          note: working.note.trim() || null,
        },
      });
      setDraft(null);
    } catch (caught) {
      if (caught instanceof ApiError) {
        const fields = caught.fieldErrors;
        if (Object.keys(fields).length > 0) setErrors(fields);
        else setFailure(caught.message);
      } else setFailure('Could not save.');
    }
  }

  return (
    <>
      <Link
        to="/stock/products/$productId"
        params={{ productId: current.product_id }}
        className="app-link"
      >
        <BackLabel>{product.data?.name ?? 'Product stock'}</BackLabel>
      </Link>
      <PageHeader
        title={product.data?.name ?? 'Stock item'}
        subtitle={current.storage_location}
      />

      <Paper variant="outlined" sx={{ p: 3, maxWidth: 560 }}>
        <form onSubmit={onSubmit}>
          <Stack spacing={3}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <StockFields
              draft={working}
              errors={errors}
              onChange={setDraft}
              lockProduct
            />
            <Stack direction="row" spacing={2}>
              <Button type="submit" variant="contained" disabled={update.isPending}>
                {update.isPending ? 'Saving…' : 'Save changes'}
              </Button>
              {!current.archived_at && (
                <Button
                  color="warning"
                  disabled={archive.isPending}
                  onClick={async () => {
                    await archive.mutateAsync({ id, revision: current.revision });
                    void navigate({
                      to: '/stock/products/$productId',
                      params: { productId: current.product_id },
                    });
                  }}
                >
                  Archive
                </Button>
              )}
            </Stack>
          </Stack>
        </form>
      </Paper>

      {canSeeHistory && (
        <Paper variant="outlined" sx={{ p: 3, mt: 3, maxWidth: 560 }}>
          <Button size="small" onClick={() => setShowHistory((open) => !open)}>
            {showHistory ? 'Hide history' : 'Show history'}
          </Button>
          {showHistory && (
            <>
              <Divider sx={{ my: 2 }} />
              <History id={id} />
            </>
          )}
        </Paper>
      )}
    </>
  );
}
