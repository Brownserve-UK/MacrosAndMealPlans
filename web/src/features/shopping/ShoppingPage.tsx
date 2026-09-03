import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import type { ShoppingRequirement } from '../../api/client';
import { useShoppingList } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { formatDayLabel, formatFullDate } from '../meal-plan/date';
import { groupBySection } from './grouping';
import { OpportunitiesPanel } from './OpportunitiesPanel';
import { RequirementCard } from './RequirementCard';
import { RequirementDialog } from './RequirementDialog';
import { requirementKey } from './requirementKey';
import { sectionLabel } from './sections';

export function ShoppingPage() {
  const [focus, setFocus] = useState<string | undefined>(undefined);
  const [showing, setShowing] = useState<ShoppingRequirement | null>(null);
  const list = useShoppingList(focus);

  const grouped = useMemo(() => groupBySection(list.data?.requirements ?? []), [list.data]);

  if (list.isLoading) return <Loading label="Working out what you need" />;
  if (list.isError) return <ErrorState error={list.error} onRetry={() => list.refetch()} />;

  const data = list.data!;
  const nextShopAfter = data.opportunities.find(
    (opportunity) => opportunity.date > (data.focus ?? ''),
  )?.date;

  return (
    <>
      <PageHeader
        title="Shopping"
        subtitle={
          data.focus
            ? `For your shop on ${formatFullDate(data.focus)}.`
            : 'Everything your plans need that your stock cannot cover.'
        }
        actions={
          <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
            {data.opportunities.length > 1 ? (
              <TextField
                select
                size="small"
                label="Shop"
                value={data.focus ?? ''}
                onChange={(e) => setFocus(e.target.value || undefined)}
                sx={{ minWidth: 180 }}
              >
                {data.opportunities.map((opportunity) => (
                  <MenuItem key={opportunity.date} value={opportunity.date}>
                    {formatDayLabel(opportunity.date)}
                    {opportunity.state === 'one_off' ? ' (extra)' : ''}
                  </MenuItem>
                ))}
              </TextField>
            ) : null}
            {data.requirements.length > 0 ? (
              <Button component={Link} to="/shopping/shop" variant="contained">
                Start shopping
              </Button>
            ) : null}
          </Stack>
        }
      />

      {!data.cadence_configured && (
        <Alert severity="info" sx={{ mb: 2.5 }}>
          Tell us when you normally shop and we'll work out what to buy for each trip. Set it up
          under Administration.
        </Alert>
      )}

      <OpportunitiesPanel opportunities={data.opportunities} />

      {data.requirements.length === 0 ? (
        <EmptyState
          title="Nothing to buy"
          description="Your stock covers everything you've planned."
        />
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
        onClose={() => setShowing(null)}
      />
    </>
  );
}
