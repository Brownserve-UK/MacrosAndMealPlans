import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type Amount, type PlannerMeal } from '../../api/client';
import { useReviewMealOutcomes } from '../../api/queries';
import type { components } from '../../api/schema';
import { FormDialog } from '../../components/FormDialog';
import { combineDateTime } from './date';

type ResultKind = 'as_planned' | 'not_eaten' | 'changed';
type ResultKinds = Record<string, ResultKind>;
type ChangedAmounts = Record<string, string>;

function amountFromSummary(summary: { kind: string; unit?: string | null; value: string }): Amount {
  const value = Number(summary.value);
  if (summary.kind === 'servings') return { kind: 'servings', value };
  if (summary.kind === 'packs') return { kind: 'packs', value };
  return { kind: 'measure', unit: summary.unit as Extract<Amount, { kind: 'measure' }>['unit'], value };
}

function changedComponents(
  subjectKey: string,
  allocations: { component_id: string; allocated: { kind: string; unit?: string | null; value: string } }[],
  values: ChangedAmounts,
) {
  return allocations.flatMap((allocation) => {
    const amount = amountFromSummary(allocation.allocated);
    const value = Number(values[`${subjectKey}:${allocation.component_id}`] ?? amount.value);
    return value > 0 ? [{ component_id: allocation.component_id, amount: { ...amount, value } }] : [];
  });
}

function outcomeFor(
  subjectKey: string,
  kind: ResultKind,
  allocations: { component_id: string; allocated: { kind: string; unit?: string | null; value: string } }[],
  values: ChangedAmounts,
): components['schemas']['ReviewedMealOutcomeRequest'] {
  if (kind === 'not_eaten') return { result: 'not_eaten' };
  if (kind === 'changed') return { result: 'changed', components: changedComponents(subjectKey, allocations, values) };
  return { result: 'as_planned' };
}

export function MealOutcomeDialog({ meal, onClose }: { meal: PlannerMeal; onClose: () => void }) {
  const review = useReviewMealOutcomes();
  const [results, setResults] = useState<ResultKinds>({});
  const [amounts, setAmounts] = useState<ChangedAmounts>({});
  const [guestExceptionCounts, setGuestExceptionCounts] = useState<Record<string, number>>({});
  const [error, setError] = useState<string | null>(null);
  const isSnack = meal.slot === 'snacks';
  const pendingPeople = meal.people.filter((person) => person.can_record && person.allocations.some((allocation) => allocation.status === 'planned'));
  const pendingGuests = meal.capabilities.can_record_guests
    ? meal.guest_groups.filter((group) => group.allocations.some((allocation) => allocation.status === 'planned'))
    : [];

  async function confirm() {
    const members = pendingPeople.map((person) => {
      const kind = results[person.member_id] ?? 'as_planned';
      const pending = person.allocations.filter((allocation) => allocation.status === 'planned');
      return { member_id: person.member_id, ...outcomeFor(person.member_id, kind, pending, amounts) } as components['schemas']['ReviewedMemberOutcomeRequest'];
    });
    const guests = pendingGuests.flatMap((group) => {
      const key = `guest:${group.id}`;
      const kind = results[key] ?? 'as_planned';
      const pending = group.allocations.filter((allocation) => allocation.status === 'planned');
      if (kind === 'as_planned') {
        return [{ source_group_id: group.id, count: group.count, result: 'as_planned' as const }];
      }
      const exceptionCount = Math.min(Math.max(guestExceptionCounts[group.id] ?? group.count, 1), group.count);
      const exception = {
        source_group_id: group.id,
        count: exceptionCount,
        ...outcomeFor(key, kind, pending, amounts),
      } as components['schemas']['ReviewedGuestOutcomeRequest'];
      return exceptionCount === group.count
        ? [exception]
        : [exception, { source_group_id: group.id, count: group.count - exceptionCount, result: 'as_planned' as const }];
    });

    try {
      await review.mutateAsync({
        id: meal.id,
        revision: meal.revision,
        body: {
          consumed_on: meal.planned_on,
          consumed_at: meal.planned_time ? combineDateTime(meal.planned_on, meal.planned_time) : null,
          members,
          guests,
        },
      });
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : `Could not record this ${isSnack ? 'snack' : 'meal'}.`);
    }
  }

  function resultFields(
    subjectKey: string,
    allocations: { component_id: string; allocated: { kind: string; unit?: string | null; value: string } }[],
  ) {
    if ((results[subjectKey] ?? 'as_planned') !== 'changed') return null;
    return (
      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5} sx={{ mt: 1.5, flexWrap: 'wrap' }}>
        {allocations.map((allocation) => {
          const food = meal.foods.find((candidate) => candidate.id === allocation.component_id);
          return (
            <TextField
              key={allocation.component_id}
              label={food?.item_name ?? 'Food'}
              type="number"
              value={amounts[`${subjectKey}:${allocation.component_id}`] ?? allocation.allocated.value}
              onChange={(event) => setAmounts((current) => ({ ...current, [`${subjectKey}:${allocation.component_id}`]: event.target.value }))}
              helperText="Set to 0 if not eaten"
              slotProps={{ htmlInput: { min: 0, step: 'any' } }}
              sx={{ width: 190 }}
            />
          );
        })}
      </Stack>
    );
  }

  return (
    <FormDialog open onClose={review.isPending ? undefined : onClose} fullWidth maxWidth="md">
      <DialogTitle>{isSnack ? 'Record snack' : 'Record meal'}</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          <Typography variant="h3">{isSnack ? 'Did everyone have their snack as planned?' : 'Did everyone eat as planned?'}</Typography>
          {error ? <Alert severity="error">{error}</Alert> : null}
          {pendingPeople.map((person) => (
            <Box key={person.member_id} sx={{ p: 2, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
              <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}>
                <Typography sx={{ fontWeight: 600 }}>{person.display_name}</Typography>
                <TextField select size="small" label="What happened?" value={results[person.member_id] ?? 'as_planned'} onChange={(event) => setResults((current) => ({ ...current, [person.member_id]: event.target.value as ResultKind }))} sx={{ minWidth: 190 }}>
                  <MenuItem value="as_planned">Ate as planned</MenuItem>
                  <MenuItem value="not_eaten">Did not eat</MenuItem>
                  <MenuItem value="changed">Ate a different amount</MenuItem>
                </TextField>
              </Stack>
              {resultFields(person.member_id, person.allocations)}
            </Box>
          ))}
          {pendingGuests.map((group) => {
            const key = `guest:${group.id}`;
            const kind = results[key] ?? 'as_planned';
            return (
              <Box key={group.id} sx={{ p: 2, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
                <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}>
                  <Typography sx={{ fontWeight: 600 }}>{group.count === 1 ? '1 guest' : `${group.count} guests`}</Typography>
                  <Stack direction="row" spacing={1.5}>
                    {kind !== 'as_planned' && group.count > 1 ? (
                      <TextField label="Guests affected" type="number" size="small" value={guestExceptionCounts[group.id] ?? group.count} onChange={(event) => setGuestExceptionCounts((current) => ({ ...current, [group.id]: Number(event.target.value) || 1 }))} slotProps={{ htmlInput: { min: 1, max: group.count, step: 1 } }} sx={{ width: 150 }} />
                    ) : null}
                    <TextField select size="small" label="What happened?" value={kind} onChange={(event) => setResults((current) => ({ ...current, [key]: event.target.value as ResultKind }))} sx={{ minWidth: 190 }}>
                      <MenuItem value="as_planned">Ate as planned</MenuItem>
                      <MenuItem value="not_eaten">Did not eat</MenuItem>
                      <MenuItem value="changed">Ate a different amount</MenuItem>
                    </TextField>
                  </Stack>
                </Stack>
                {resultFields(key, group.allocations)}
              </Box>
            );
          })}
          {pendingPeople.length === 0 && pendingGuests.length === 0 ? <Alert severity="info">This {isSnack ? 'snack' : 'meal'} has already been recorded.</Alert> : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={review.isPending}>Cancel</Button>
        <Button variant="contained" onClick={() => void confirm()} disabled={review.isPending || (pendingPeople.length === 0 && pendingGuests.length === 0)}>Confirm {isSnack ? 'snack' : 'meal'}</Button>
      </DialogActions>
    </FormDialog>
  );
}
