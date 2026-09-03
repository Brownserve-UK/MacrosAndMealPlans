import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import type { ShoppingRequirement } from '../../api/client';
import { useRecordPurchase, useShoppingList, useUpdatePurchase } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { formatFullDate } from '../meal-plan/date';
import { groupBySection } from './grouping';
import { RequirementCard } from './RequirementCard';
import { RequirementDialog } from './RequirementDialog';
import { requirementKey } from './requirementKey';
import { sectionLabel } from './sections';

export function ShopModePage() {
  const [showing, setShowing] = useState<ShoppingRequirement | null>(null);
  const list = useShoppingList(undefined);
  const record = useRecordPurchase();
  const update = useUpdatePurchase();

  const grouped = useMemo(() => groupBySection(list.data?.requirements ?? []), [list.data]);

  if (list.isLoading) return <Loading label="Fetching your list" />;
  if (list.isError) return <ErrorState error={list.error} onRetry={() => list.refetch()} />;

  const data = list.data!;
  const nextShopAfter = data.opportunities.find(
    (opportunity) => opportunity.date > (data.focus ?? ''),
  )?.date;
  const bought = data.requirements.filter((requirement) => requirement.purchase != null).length;

  function toggle(requirement: ShoppingRequirement, next: boolean) {
    const purchase = requirement.purchase;
    if (next) {
      record.mutate({
        ingredient_id:
          requirement.subject.kind === 'ingredient' ? requirement.subject.ingredient_id : undefined,
        product_id:
          requirement.subject.kind === 'product' ? requirement.subject.product_id : undefined,
        opportunity_date: data.focus ?? undefined,
      });
    } else if (purchase) {
      update.mutate({ id: purchase.id, revision: purchase.revision, cancelled: true });
    }
  }

  return (
    <>
      <PageHeader
        title={data.focus ? formatFullDate(data.focus) : 'Shopping'}
        subtitle={`${bought} of ${data.requirements.length} in the trolley`}
        actions={
          <Button component={Link} to="/shopping" variant="outlined">
            Done
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
                    bought={requirement.purchase != null}
                    onToggle={(next) => toggle(requirement, next)}
                    onOpen={() => setShowing(requirement)}
                  />
                </div>
              ))}
            </Paper>
          ))}
        </Stack>
      )}

      <RequirementDialog
        open={showing != null}
        requirement={showing}
        opportunityDate={data.focus}
        buying={showing?.purchase != null}
        onClose={() => setShowing(null)}
      />
    </>
  );
}
