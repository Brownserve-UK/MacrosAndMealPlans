import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import { useNavigate, useParams } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { ApiError } from '../../api/client';
import type { ShoppingListItem, ShoppingRequirement } from '../../api/client';
import {
  useAddShoppingListItem,
  useAbandonShop,
  useFinishShop,
  useDismissShoppingSuggestion,
  useRecordPurchase,
  useRemoveShoppingListItem,
  useShoppingList,
  useStartShop,
  useUpdatePurchase,
  useUpdateShoppingListItem,
} from '../../api/queries';
import { Link } from '@tanstack/react-router';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { formatDayLabel, formatFullDate } from '../meal-plan/date';
import { AddAnything } from './AddAnything';
import { pinnedList } from './baseline';
import { groupBySection, isSuggested } from './grouping';
import { ManualRow } from './ManualRow';
import { RequirementCard } from './RequirementCard';
import { SuggestedRow } from './SuggestedRow';
import { RequirementDialog } from './RequirementDialog';
import { purchasesOf, requirementKey } from './requirementKey';
import { sectionLabel } from './sections';

export function TripPage() {
  const { date } = useParams({ from: '/shopping/$date' });
  const list = useShoppingList(date);
  const start = useStartShop();
  const abandon = useAbandonShop();
  const finish = useFinishShop();
  const record = useRecordPurchase();
  const update = useUpdatePurchase();
  const updateItem = useUpdateShoppingListItem();
  const dismiss = useDismissShoppingSuggestion();
  const removeItem = useRemoveShoppingListItem();
  const addItem = useAddShoppingListItem();
  const navigate = useNavigate();

  const [showingKey, setShowingKey] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [abandoning, setAbandoning] = useState(false);
  const [editing, setEditing] = useState<ShoppingListItem | null>(null);
  const [editName, setEditName] = useState('');
  const [conflict, setConflict] = useState<ApiError | null>(null);

  const pinned = useMemo(() => (list.data ? pinnedList(list.data) : null), [list.data]);

  const grouped = useMemo(
    () =>
      groupBySection(
        (
          pinned?.rows ??
          (list.data?.requirements ?? []).map((requirement) => ({
            key: requirementKey(requirement),
            requirement,
          }))
        ),
        list.data?.manual ?? [],
        list.data?.section_order,
      ),
    [list.data, pinned],
  );

  if (list.isLoading) return <Loading label="Fetching your list" />;
  if (list.isError) return <ErrorState error={list.error} onRetry={() => list.refetch()} />;

  if (!list.data) return <Loading label="Fetching your list" />;
  const data = list.data;
  const listed = pinned
    ? [...pinned.rows.map((row) => row.requirement), ...pinned.added]
    : data.requirements;
  const showing = listed.find((requirement) => requirementKey(requirement) === showingKey) ?? null;
  const underway = data.trip != null && data.trip.state === 'shopping';
  const trolley = listed.flatMap(purchasesOf).length + data.unplanned.length;
  const total = listed.length + data.manual.length;
  const ready = [...data.requirements.flatMap(purchasesOf), ...data.unplanned].filter(
    (purchase) => purchase.product_id && purchase.quantity,
  ).length;

  function tick(requirement: ShoppingRequirement, next: boolean) {
    if (!underway) void start.mutateAsync(date);
    if (next) {
      record.mutate({
        ingredient_id:
          requirement.subject.kind === 'ingredient' ? requirement.subject.ingredient_id : undefined,
        product_id:
          requirement.subject.kind === 'product' ? requirement.subject.product_id : undefined,
        opportunity_date: date,
      });
      return;
    }
    for (const purchase of purchasesOf(requirement)) {
      update.mutate({ id: purchase.id, revision: purchase.revision, cancelled: true });
    }
  }

  function boughtManually(item: ShoppingListItem) {
    return data.unplanned.filter((purchase) => {
      if (purchase.state === 'cancelled') return false;
      if (item.product_id) return purchase.product_id === item.product_id;
      if (item.ingredient_id) return purchase.ingredient_id === item.ingredient_id;
      return purchase.name === item.name;
    });
  }

  function tickManual(item: ShoppingListItem, next: boolean) {
    if (!underway) void start.mutateAsync(date);
    if (next) {
      record.mutate({
        ingredient_id: item.ingredient_id ?? undefined,
        product_id: item.product_id ?? undefined,
        name: item.product_id || item.ingredient_id ? undefined : item.name,
        opportunity_date: date,
      });
      return;
    }
    for (const purchase of boughtManually(item)) {
      update.mutate({ id: purchase.id, revision: purchase.revision, cancelled: true });
    }
  }

  function onAddSuggested(requirement: ShoppingRequirement) {
    addItem.mutate({
      name: requirement.name,
      ingredient_id:
        requirement.subject.kind === 'ingredient' ? requirement.subject.ingredient_id : undefined,
      product_id:
        requirement.subject.kind === 'product' ? requirement.subject.product_id : undefined,
      section: requirement.section,
      opportunity_date: date,
    });
  }

  function onDismiss(requirement: ShoppingRequirement) {
    const subject = requirement.subject;
    if (subject.kind === 'ingredient') {
      dismiss.mutate({ date, kind: subject.kind, id: subject.ingredient_id });
    } else if (subject.kind === 'prepared_meal') {
      dismiss.mutate({ date, kind: subject.kind, id: subject.prepared_meal_id });
    } else if (subject.kind === 'product') {
      dismiss.mutate({ date, kind: subject.kind, id: subject.product_id });
    }
  }

  function beginEdit(item: ShoppingListItem) {
    setEditing(item);
    setEditName(item.name);
  }

  async function saveEdit() {
    if (!editing || !editName.trim()) return;
    await updateItem.mutateAsync({
      id: editing.id,
      revision: editing.revision,
      name: editName.trim(),
    });
    setEditing(null);
  }

  async function onFinish() {
    try {
      await finish.mutateAsync({ date, revision: data.trip?.revision ?? 0 });
      setFinishing(false);
      void navigate({ to: ready > 0 ? '/shopping/put-away' : '/shopping' });
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) {
        setFinishing(false);
        setConflict(caught);
      }
    }
  }

  async function onAbandon() {
    try {
      await abandon.mutateAsync({ date, revision: data.trip?.revision ?? 0 });
      setAbandoning(false);
      void list.refetch();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) {
        setAbandoning(false);
        setConflict(caught);
      }
    }
  }

  return (
    <>
      <PageHeader
        back={
          <Link to="/shopping" className="app-link">
            <BackLabel>Shopping</BackLabel>
          </Link>
        }
        title={formatDayLabel(date)}
        subtitle={formatFullDate(date)}
        actions={
          underway ? (
            <Button onClick={() => setAbandoning(true)}>Abandon trip</Button>
          ) : (
            <Button variant="contained" onClick={() => void start.mutateAsync(date)}>
              Start shopping
            </Button>
          )
        }
      />

      <Stack spacing={2.5} sx={{ pb: underway ? 10 : 0 }}>
        <AddAnything date={date} />

        {total === 0 ? (
          <Typography variant="body2" color="text.secondary">
            Nothing to buy for this shop.
          </Typography>
        ) : null}

        {grouped.map(([section, rows]) => (
          <Paper key={section} variant="outlined" sx={{ overflow: 'hidden' }}>
            <Typography
              variant="overline"
              sx={{ px: 2, pt: 1.5, pb: 1, display: 'block', color: 'text.secondary' }}
            >
              {sectionLabel(section as never)}
            </Typography>
            {rows.map((row, index) => {
              if (row.kind === 'manual') {
                const item = row.item;
                return (
                  <ManualRow
                    key={item.id}
                    item={item}
                    bought={boughtManually(item).length > 0}
                    onToggle={(next) => tickManual(item, next)}
                    onEdit={() => beginEdit(item)}
                  />
                );
              }

              const previous = rows[index - 1];
              const opensGroup =
                isSuggested(row.requirement) &&
                (previous == null ||
                  previous.kind !== 'requirement' ||
                  !isSuggested(previous.requirement));

              const requirement = row.requirement;
              return (
                <Box key={row.key}>
                  {opensGroup ? (
                    <Typography
                      variant="overline"
                      sx={{
                        px: 2,
                        pt: 1.5,
                        pb: 0.5,
                        display: 'block',
                        color: 'text.disabled',
                        borderTop: 1,
                        borderColor: 'divider',
                      }}
                    >
                      Suggested
                    </Typography>
                  ) : null}
                  {isSuggested(requirement) ? (
                    <SuggestedRow
                      requirement={requirement}
                      onAdd={() => onAddSuggested(requirement)}
                      onDismiss={() => onDismiss(requirement)}
                    />
                  ) : (
                    <RequirementCard
                      requirement={requirement}
                      bought={purchasesOf(requirement).length > 0}
                      onToggle={(next) => tick(requirement, next)}
                      onOpen={() => setShowingKey(requirementKey(requirement))}
                    />
                  )}
                </Box>
              );
            })}
          </Paper>
        ))}

        {pinned && pinned.added.length > 0 ? (
          <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
            <Box sx={{ px: 2, pt: 1.5, pb: 1 }}>
              <Typography variant="overline" sx={{ display: 'block', color: 'text.secondary' }}>
                Since you started
              </Typography>
              <Typography variant="caption" color="text.secondary">
                The plan changed. Your list above is as you left it.
              </Typography>
            </Box>
            {pinned.added.map((requirement) => (
              <RequirementCard
                key={requirementKey(requirement)}
                requirement={requirement}
                bought={purchasesOf(requirement).length > 0}
                onToggle={(next) => tick(requirement, next)}
                onOpen={() => setShowingKey(requirementKey(requirement))}
              />
            ))}
          </Paper>
        ) : null}
      </Stack>

      {underway ? (
        <Paper
          variant="outlined"
          sx={{
            position: 'sticky',
            bottom: 0,
            mt: 2,
            px: 2,
            py: 1.5,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: 2,
          }}
        >
          <Typography variant="body2" color="text.secondary" className="numeral">
            {trolley} of {total} in the trolley
          </Typography>
          <Button variant="contained" onClick={() => setFinishing(true)}>
            Finish shop
          </Button>
        </Paper>
      ) : null}

      <FormDialog open={finishing} onClose={() => setFinishing(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Finish shop</DialogTitle>
        <DialogContent>
          <Typography variant="body1">
            {ready === 1 ? '1 thing goes into your stock.' : `${ready} things go into your stock.`}
          </Typography>
          {trolley - ready > 0 ? (
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              The rest waits until you put it away.
            </Typography>
          ) : null}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setFinishing(false)}>Cancel</Button>
          <Button variant="contained" disabled={finish.isPending} onClick={() => void onFinish()}>
            {finish.isPending ? 'Saving…' : 'Finish'}
          </Button>
        </DialogActions>
      </FormDialog>

      <FormDialog open={abandoning} onClose={() => setAbandoning(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Abandon trip</DialogTitle>
        <DialogContent>
          <Typography variant="body1">
            Your list will go live again. Anything already in the trolley stays bought and can be
            put away or cancelled here.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setAbandoning(false)}>Keep shopping</Button>
          <Button disabled={abandon.isPending} onClick={() => void onAbandon()}>
            {abandon.isPending ? 'Saving…' : 'Abandon trip'}
          </Button>
        </DialogActions>
      </FormDialog>

      <FormDialog open={editing != null} onClose={() => setEditing(null)} maxWidth="xs" fullWidth>
        <DialogTitle>Edit item</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            fullWidth
            label="Item"
            value={editName}
            onChange={(event) => setEditName(event.target.value)}
          />
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'space-between' }}>
          <Button
            disabled={removeItem.isPending}
            onClick={() => {
              if (editing) {
                removeItem.mutate({ id: editing.id, revision: editing.revision });
                setEditing(null);
              }
            }}
          >
            Remove
          </Button>
          <Stack direction="row" spacing={1}>
            <Button onClick={() => setEditing(null)}>Cancel</Button>
            <Button
              variant="contained"
              disabled={updateItem.isPending || !editName.trim()}
              onClick={() => void saveEdit()}
            >
              Save
            </Button>
          </Stack>
        </DialogActions>
      </FormDialog>

      <RequirementDialog
        open={showing != null}
        requirement={showing}
        opportunityDate={date}
        buying
        onClose={() => setShowingKey(null)}
      />

      <ConflictDialog
        error={conflict}
        onReload={() => {
          setConflict(null);
          void list.refetch();
        }}
        onDismiss={() => setConflict(null)}
      />
    </>
  );
}
