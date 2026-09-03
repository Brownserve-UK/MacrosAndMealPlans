import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import type { ShoppingRequirement } from '../../api/client';
import {
  useFinishShop,
  useRecordPurchase,
  useShoppingList,
  useUpdatePurchase,
} from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { formatFullDate } from '../meal-plan/date';
import { groupBySection } from './grouping';
import { RequirementCard } from './RequirementCard';
import { RequirementDialog } from './RequirementDialog';
import { purchasesOf, requirementKey } from './requirementKey';
import { sectionLabel } from './sections';

export function ShopModePage() {
  const [showingKey, setShowingKey] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);
  const navigate = useNavigate();
  const list = useShoppingList(undefined);
  const record = useRecordPurchase();
  const update = useUpdatePurchase();
  const finish = useFinishShop();

  const grouped = useMemo(() => groupBySection(list.data?.requirements ?? []), [list.data]);

  if (list.isLoading) return <Loading label="Fetching your list" />;
  if (list.isError) return <ErrorState error={list.error} onRetry={() => list.refetch()} />;

  const data = list.data!;
  const showing =
    data.requirements.find((requirement) => requirementKey(requirement) === showingKey) ?? null;
  const nextShopAfter = data.opportunities.find(
    (opportunity) => opportunity.date > (data.focus ?? ''),
  )?.date;
  const trolley = data.requirements.flatMap(purchasesOf);
  const bought = data.requirements.filter(
    (requirement) => purchasesOf(requirement).length > 0,
  ).length;
  const ready = trolley.filter((purchase) => purchase.product_id && purchase.quantity).length;
  const waiting = trolley.length - ready;

  async function onFinish() {
    if (!data.focus) return;
    await finish.mutateAsync(data.focus);
    setFinishing(false);
    void navigate({ to: '/shopping' });
  }

  function toggle(requirement: ShoppingRequirement, next: boolean) {
    const purchases = purchasesOf(requirement);
    if (next) {
      record.mutate({
        ingredient_id:
          requirement.subject.kind === 'ingredient' ? requirement.subject.ingredient_id : undefined,
        product_id:
          requirement.subject.kind === 'product' ? requirement.subject.product_id : undefined,
        opportunity_date: data.focus ?? undefined,
      });
    } else {
      for (const purchase of purchases) {
        update.mutate({ id: purchase.id, revision: purchase.revision, cancelled: true });
      }
    }
  }

  return (
    <>
      <PageHeader
        title={data.focus ? formatFullDate(data.focus) : 'Shopping'}
        subtitle={`${bought} of ${data.requirements.length} in the trolley`}
        actions={
          <Button
            variant="contained"
            disabled={trolley.length === 0 || !data.focus}
            onClick={() => setFinishing(true)}
          >
            Finish shop
          </Button>
        }
      />

      {data.requirements.length === 0 ? (
        <EmptyState title="Nothing to buy" description="Your stock covers everything." />
      ) : (
        <Stack spacing={2.5}>
          {grouped.map(([section, requirements]) => (
            <Paper key={section} variant="outlined" sx={{ overflow: 'hidden' }}>
              <Typography
                variant="overline"
                sx={{ px: 2, pt: 1.5, pb: 1, display: 'block', color: 'text.secondary' }}
              >
                {sectionLabel(section as never)}
              </Typography>
              <Divider />
              {requirements.map((requirement, index) => (
                <div key={requirementKey(requirement)}>
                  {index > 0 ? <Divider /> : null}
                  <RequirementCard
                    requirement={requirement}
                    nextShopAfter={nextShopAfter}
                    bought={purchasesOf(requirement).length > 0}
                    onToggle={(next) => toggle(requirement, next)}
                    onOpen={() => setShowingKey(requirementKey(requirement))}
                  />
                </div>
              ))}
            </Paper>
          ))}
        </Stack>
      )}

      <FormDialog open={finishing} onClose={() => setFinishing(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Finish shop</DialogTitle>
        <DialogContent>
          <Typography variant="body1">
            {ready === 1 ? '1 item goes into your stock.' : `${ready} items go into your stock.`}
          </Typography>
          {waiting > 0 ? (
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              {waiting === 1
                ? '1 still needs details and will wait.'
                : `${waiting} still need details and will wait.`}
            </Typography>
          ) : null}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setFinishing(false)}>Cancel</Button>
          <Button
            variant="contained"
            disabled={finish.isPending}
            onClick={() => void onFinish()}
          >
            {finish.isPending ? 'Saving…' : 'Finish'}
          </Button>
        </DialogActions>
      </FormDialog>

      <RequirementDialog
        open={showing != null}
        requirement={showing}
        opportunityDate={data.focus}
        buying
        onClose={() => setShowingKey(null)}
      />
    </>
  );
}
