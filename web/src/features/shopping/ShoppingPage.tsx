import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { ShoppingOpportunity } from '../../api/client';
import { usePendingPutAway, useShoppingList } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { useHouseholdTimeZone } from '../../hooks/useHouseholdTimeZone';
import { todayIso } from '../meal-plan/date';
import { ChangeShopDialog } from './ChangeShopDialog';
import { GapCard } from './GapCard';
import { TripCard } from './TripCard';

export function ShoppingPage() {
  const list = useShoppingList(undefined);
  const waiting = usePendingPutAway();
  const timeZone = useHouseholdTimeZone();
  const [changing, setChanging] = useState<ShoppingOpportunity | null>(null);

  if (list.isLoading) return <Loading label="Working out what you need" />;
  if (list.isError) return <ErrorState error={list.error} onRetry={() => list.refetch()} />;

  const data = list.data!;
  const unpacked = waiting.data ?? [];
  const countFor = (date: string) =>
    data.counts.find((count) => count.date === date)?.items ?? 0;

  const [next, ...rest] = data.opportunities;
  const later = rest.filter((opportunity) => countFor(opportunity.date) > 0);
  const soonest = data.requirements.find(
    (requirement) => requirement.assignment.kind === 'needs_earlier_opportunity',
  );

  const sections: string[] = [];
  for (const requirement of data.requirements) {
    if (!sections.includes(requirement.section)) sections.push(requirement.section);
  }

  return (
    <>
      <PageHeader title="Shopping" />

      <Stack spacing={2}>
        {unpacked.length > 0 ? (
          <Paper variant="outlined" sx={{ p: 2.5 }}>
            <Stack
              direction={{ xs: 'column', sm: 'row' }}
              spacing={2}
              sx={{ alignItems: { sm: 'center' } }}
            >
              <Box sx={{ flexGrow: 1, minWidth: 0 }}>
                <Typography variant="h2">Put the shopping away</Typography>
                <Typography variant="body2" color="text.secondary" className="numeral">
                  {unpacked.length === 1
                    ? '1 thing, still in the bags'
                    : `${unpacked.length} things, still in the bags`}
                </Typography>
              </Box>
              <Box sx={{ flexShrink: 0 }}>
                <Link to="/shopping/put-away" className="app-link">
                  <Button variant="contained" component="span">
                    Put it away
                  </Button>
                </Link>
              </Box>
            </Stack>
          </Paper>
        ) : null}

        {next ? (
          <TripCard
            opportunity={next}
            count={countFor(next.date)}
            sections={sections}
            imminent
            onChange={() => setChanging(next)}
          />
        ) : null}

        {soonest ? <GapCard requirement={soonest} /> : null}

        {later.map((opportunity) => (
          <TripCard
            key={opportunity.date}
            opportunity={opportunity}
            count={countFor(opportunity.date)}
            onChange={() => setChanging(opportunity)}
          />
        ))}

        {data.opportunities.length === 0 ? (
          <Typography variant="body2" color="text.secondary">
            {data.cadence_configured
              ? 'No shops coming up.'
              : 'Tell us when you shop and we will work out what to buy for each trip.'}
          </Typography>
        ) : null}

        <Box>
          <Button component={Link} to="/administration/shopping" sx={{ ml: -1.5 }}>
            {data.cadence_configured ? 'Change when you shop' : 'Set up your shopping days'}
          </Button>
        </Box>
      </Stack>

      <ChangeShopDialog
        date={changing ? (changing.generated_for ?? changing.date) : null}
        revision={changing?.revision ?? 0}
        earliest={todayIso(timeZone)}
        onClose={() => setChanging(null)}
      />
    </>
  );
}
