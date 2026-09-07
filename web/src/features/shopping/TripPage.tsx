import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate, useParams } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import type { ShoppingRequirement } from '../../api/client';
import {
  useAddShoppingListItem,
  useFinishShop,
  useRecordPurchase,
  useRemoveShoppingListItem,
  useShoppingList,
  useStartShop,
  useUpdatePurchase,
} from '../../api/queries';
import { Link } from '@tanstack/react-router';
import { BackLabel } from '../../components/BackLink';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { formatDayLabel, formatFullDate } from '../meal-plan/date';
import { AddAnything } from './AddAnything';
import { groupBySection, isSuggested } from './grouping';
import { RequirementCard } from './RequirementCard';
import { SuggestedRow } from './SuggestedRow';
import { RequirementDialog } from './RequirementDialog';
import { purchasesOf, requirementKey } from './requirementKey';
import { sectionLabel } from './sections';

export function TripPage() {
  const { date } = useParams({ from: '/shopping/$date' });
  const list = useShoppingList(date);
  const start = useStartShop();
  const finish = useFinishShop();
  const record = useRecordPurchase();
  const update = useUpdatePurchase();
  const removeItem = useRemoveShoppingListItem();
  const addItem = useAddShoppingListItem();
  const navigate = useNavigate();

  const [showingKey, setShowingKey] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [dismissed, setDismissed] = useState<string[]>([]);

  const grouped = useMemo(
    () =>
      groupBySection(
        (list.data?.requirements ?? []).filter(
          (requirement) => !dismissed.includes(requirementKey(requirement)),
        ),
      ),
    [list.data, dismissed],
  );

  if (list.isLoading) return <Loading label="Fetching your list" />;
  if (list.isError) return <ErrorState error={list.error} onRetry={() => list.refetch()} />;

  const data = list.data!;
  const showing =
    data.requirements.find((requirement) => requirementKey(requirement) === showingKey) ?? null;
  const underway = data.trip != null && data.trip.state === 'shopping';
  const trolley = data.requirements.flatMap(purchasesOf).length + data.unplanned.length;
  const total = data.requirements.length + data.manual.length;
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
    setDismissed((held) => [...held, requirementKey(requirement)]);
  }

  async function onFinish() {
    await finish.mutateAsync(date);
    setFinishing(false);
    void navigate({ to: ready > 0 ? '/shopping/put-away' : '/shopping' });
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
          underway ? null : (
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

        {grouped.map(([section, requirements]) => (
          <Paper key={section} variant="outlined" sx={{ overflow: 'hidden' }}>
            <Typography
              variant="overline"
              sx={{ px: 2, pt: 1.5, pb: 1, display: 'block', color: 'text.secondary' }}
            >
              {sectionLabel(section as never)}
            </Typography>
            {requirements.map((requirement, index) => (
              <Box key={requirementKey(requirement)}>
                {isSuggested(requirement) &&
                (index === 0 || !isSuggested(requirements[index - 1] as ShoppingRequirement)) ? (
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
            ))}
          </Paper>
        ))}

        {data.manual.length > 0 ? (
          <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
            <Typography
              variant="overline"
              sx={{ px: 2, pt: 1.5, pb: 1, display: 'block', color: 'text.secondary' }}
            >
              You added
            </Typography>
            {data.manual.map((item) => (
              <Stack
                key={item.id}
                direction="row"
                spacing={1}
                sx={{ alignItems: 'center', px: 2, py: 1.25, borderTop: 1, borderColor: 'divider' }}
              >
                <Typography variant="body1" sx={{ flexGrow: 1, fontWeight: 500 }}>
                  {item.name}
                </Typography>
                <Button size="small" onClick={() => removeItem.mutate(item.id)}>
                  Remove
                </Button>
              </Stack>
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

      <RequirementDialog
        open={showing != null}
        requirement={showing}
        opportunityDate={date}
        buying
        onClose={() => setShowingKey(null)}
      />

      <Box />
    </>
  );
}
