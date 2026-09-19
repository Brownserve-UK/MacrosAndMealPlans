import AddIcon from '@mui/icons-material/Add';
import RemoveIcon from '@mui/icons-material/Remove';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type Amount, type MealPlanEntry } from '../../api/client';
import { usePlacePortions, useReviewMealOutcomes } from '../../api/queries';
import type { components } from '../../api/schema';
import { ConceptIcon } from '../../components/ConceptIcon';
import { FormDialog } from '../../components/FormDialog';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { combineDateTime } from './date';

type AmountSummary = { kind: string; unit?: string | null; value: string };
type Allocation = { component_id: string; allocated: AmountSummary };
type Claims = Record<string, number>;

const STEP = 0.5;

function amountFromSummary(summary: AmountSummary, value: number): Amount {
  if (summary.kind === 'servings') return { kind: 'servings', value };
  if (summary.kind === 'packs') return { kind: 'packs', value };
  return { kind: 'measure', unit: summary.unit as Extract<Amount, { kind: 'measure' }>['unit'], value };
}

function show(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

function servingsMade(food: MealPlanEntry['components'][number]): number | null {
  if (food.amount.kind !== 'servings') return null;
  return food.cooked ? food.cooked.servings_produced : Number(food.amount.value);
}

export function MealOutcomeDialog({
  meal,
  canRecord,
  canRecordGuests,
  onClose,
}: {
  meal: MealPlanEntry;
  canRecord: boolean;
  canRecordGuests: boolean;
  onClose: () => void;
}) {
  const review = useReviewMealOutcomes();
  const place = usePlacePortions();
  const [claims, setClaims] = useState<Claims>({});
  const [fridge, setFridge] = useState<Record<string, number>>({});
  const [error, setError] = useState<string | null>(null);
  const isSnack = meal.slot === 'snacks';

  const pendingPeople = meal.participants.filter(
    (person) => canRecord && person.allocations.some((allocation) => allocation.status === 'planned'),
  );
  const pendingGuests = canRecordGuests
    ? meal.guest_groups.filter((group) => group.allocations.some((allocation) => allocation.status === 'planned'))
    : [];

  const subjects = [
    ...pendingPeople.map((person) => ({
      key: person.member_id,
      name: person.display_name,
      avatarName: person.display_name,
      heads: 1,
      allocations: person.allocations.filter((a) => a.status === 'planned') as Allocation[],
    })),
    ...pendingGuests.map((group) => ({
      key: `guest:${group.id}`,
      name: group.name ?? (group.count === 1 ? 'Guest' : `${group.count} guests`),
      avatarName: group.name ?? 'Guest',
      heads: group.count,
      allocations: group.allocations.filter((a) => a.status === 'planned') as Allocation[],
    })),
  ];

  function claimKey(subjectKey: string, componentId: string) {
    return `${subjectKey}:${componentId}`;
  }

  const defaults: Claims = {};
  const spare = new Map<string, number>();
  for (const food of meal.components) {
    const made = servingsMade(food);
    if (made != null) spare.set(food.id, made);
  }
  for (const subject of subjects) {
    for (const allocation of subject.allocations) {
      const wanted = Number(allocation.allocated.value);
      const left = spare.get(allocation.component_id);
      if (left == null) {
        defaults[claimKey(subject.key, allocation.component_id)] = wanted;
        continue;
      }
      const given = Math.max(0, Math.min(wanted, left / subject.heads));
      defaults[claimKey(subject.key, allocation.component_id)] = given;
      spare.set(allocation.component_id, left - given * subject.heads);
    }
  }

  function claimFor(subjectKey: string, allocation: Allocation) {
    const key = claimKey(subjectKey, allocation.component_id);
    return claims[key] ?? defaults[key] ?? 0;
  }

  function claimedTotal(componentId: string) {
    return subjects.reduce((total, subject) => {
      const allocation = subject.allocations.find((a) => a.component_id === componentId);
      if (!allocation) return total;
      return total + claimFor(subject.key, allocation) * subject.heads;
    }, 0);
  }

  function remainingFor(componentId: string) {
    const food = meal.components.find((candidate) => candidate.id === componentId);
    const made = food ? servingsMade(food) : null;
    if (made == null) return null;
    return Math.round((made - claimedTotal(componentId)) * 10) / 10;
  }

  function setClaim(subjectKey: string, componentId: string, value: number) {
    setClaims((current) => ({ ...current, [claimKey(subjectKey, componentId)]: Math.max(0, value) }));
  }

  async function confirm() {
    const outcomeFor = (subject: (typeof subjects)[number]) => {
      const components_ = subject.allocations.flatMap((allocation) => {
        const value = claimFor(subject.key, allocation);
        return value > 0
          ? [{ component_id: allocation.component_id, amount: amountFromSummary(allocation.allocated, value) }]
          : [];
      });
      return components_.length === 0
        ? ({ result: 'not_eaten' } as const)
        : ({ result: 'changed', components: components_ } as const);
    };

    const members = pendingPeople.map((person) => {
      const subject = subjects.find((candidate) => candidate.key === person.member_id)!;
      return {
        member_id: person.member_id,
        ...outcomeFor(subject),
      } as components['schemas']['ReviewedMemberOutcomeRequest'];
    });
    const guests = pendingGuests.map((group) => {
      const subject = subjects.find((candidate) => candidate.key === `guest:${group.id}`)!;
      return {
        source_group_id: group.id,
        count: group.count,
        ...outcomeFor(subject),
      } as components['schemas']['ReviewedGuestOutcomeRequest'];
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

      for (const food of meal.components) {
        const remaining = remainingFor(food.id);
        if (!food.cooked || remaining == null || remaining <= 0) continue;
        const chilled = Math.min(fridge[food.id] ?? 0, remaining);
        const frozen = Math.round((remaining - chilled) * 10) / 10;
        const placements = [
          ...(chilled > 0 ? [{ storage_location: 'chilled' as const, servings: chilled }] : []),
          ...(frozen > 0 ? [{ storage_location: 'frozen' as const, servings: frozen }] : []),
        ];
        if (placements.length === 0) continue;
        await place.mutateAsync({
          id: food.cooked.prepared_batch_id,
          revision: food.cooked.revision,
          body: { placements },
        });
      }
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : `Could not record this ${isSnack ? 'snack' : 'meal'}.`);
    }
  }

  const madeLine = meal.components
    .map((food) => {
      const made = servingsMade(food);
      return made == null ? null : `${show(made)} ${made === 1 ? 'serving' : 'servings'} of ${food.item_name} made`;
    })
    .filter(Boolean)
    .join(' · ');

  return (
    <FormDialog open onClose={review.isPending ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">{isSnack ? 'Record snack' : 'Record meal'}</Typography>
        {madeLine ? (
          <Typography component="span" variant="body2" color="text.secondary" className="numeral" sx={{ display: 'block', mt: 0.5 }}>
            {madeLine}
          </Typography>
        ) : null}
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={1.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          {subjects.map((subject) => {
            const ateNothing = subject.allocations.every((a) => claimFor(subject.key, a) === 0);
            const caption = ateNothing ? 'Did not eat' : subject.heads > 1 ? 'Each' : null;
            return { ...subject, caption };
          }).map((subject) => (
            <Box
              key={subject.key}
              sx={{ p: 1.75, border: '1px solid', borderColor: 'divider', borderRadius: 2.5 }}
            >
              <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
                <InitialsAvatar name={subject.avatarName} size={34} />
                <Box sx={{ flexGrow: 1, minWidth: 0 }}>
                  <Typography sx={{ fontWeight: 500 }}>{subject.name}</Typography>
                  {subject.caption ? (
                    <Typography variant="caption" color="text.secondary">{subject.caption}</Typography>
                  ) : null}
                </Box>
                <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
                  {subject.allocations.map((allocation) => {
                    const food = meal.components.find((candidate) => candidate.id === allocation.component_id);
                    const value = claimFor(subject.key, allocation);
                    if (allocation.allocated.kind !== 'servings') {
                      return (
                        <TextField
                          key={allocation.component_id}
                          label={food?.item_name ?? 'Food'}
                          type="number"
                          size="small"
                          value={String(value)}
                          onChange={(event) =>
                            setClaim(subject.key, allocation.component_id, Number(event.target.value) || 0)
                          }
                          slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                          sx={{ width: 150 }}
                        />
                      );
                    }
                    const remaining = remainingFor(allocation.component_id);
                    return (
                      <Stack
                        key={allocation.component_id}
                        direction="row"
                        spacing={0.5}
                        sx={{ alignItems: 'center' }}
                      >
                        <IconButton
                          size="small"
                          aria-label={`Less for ${subject.name}`}
                          disabled={value < STEP}
                          onClick={() => setClaim(subject.key, allocation.component_id, value - STEP)}
                        >
                          <RemoveIcon fontSize="small" />
                        </IconButton>
                        <Typography className="numeral" sx={{ minWidth: 28, textAlign: 'center', fontWeight: 600 }}>
                          {show(value)}
                        </Typography>
                        <IconButton
                          size="small"
                          aria-label={`More for ${subject.name}`}
                          disabled={remaining != null && remaining < STEP * subject.heads}
                          onClick={() => setClaim(subject.key, allocation.component_id, value + STEP)}
                        >
                          <AddIcon fontSize="small" />
                        </IconButton>
                      </Stack>
                    );
                  })}
                </Stack>
              </Stack>
            </Box>
          ))}

          {meal.components.map((food) => {
            const remaining = remainingFor(food.id);
            if (remaining == null) return null;
            return (
              <Box
                key={food.id}
                sx={{
                  borderRadius: 2.5,
                  px: 1.75,
                  py: 1.5,
                  bgcolor: remaining > 0 ? 'action.selected' : 'action.hover',
                }}
              >
                <Typography variant="subtitle2" className="numeral">
                  {remaining > 0 ? `${show(remaining)} left over` : 'Nothing left over'}
                </Typography>
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                  {remaining > 0 ? 'Put it away' : `All of the ${food.item_name} was eaten`}
                </Typography>
                {remaining > 0 && food.cooked ? (
                  <Stack direction="row" spacing={2} sx={{ mt: 1.25, flexWrap: 'wrap' }}>
                    {(['chilled', 'frozen'] as const).map((where) => {
                      const chilled = Math.min(fridge[food.id] ?? 0, remaining);
                      const value = where === 'chilled'
                        ? chilled
                        : Math.round((remaining - chilled) * 10) / 10;
                      const label = where === 'chilled' ? 'Fridge' : 'Freezer';
                      const move = (next: number) =>
                        setFridge((now) => ({
                          ...now,
                          [food.id]: Math.max(0, Math.min(remaining, next)),
                        }));
                      return (
                        <Stack key={where} direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
                          <ConceptIcon concept={where === 'chilled' ? 'fridge' : 'freezer'} size={16} />
                          <Typography variant="body2">{label}</Typography>
                          <IconButton
                            size="small"
                            aria-label={`Less in the ${label.toLowerCase()}`}
                            disabled={value <= 0}
                            onClick={() => move(where === 'chilled' ? chilled - STEP : chilled + STEP)}
                          >
                            <RemoveIcon fontSize="small" />
                          </IconButton>
                          <Typography className="numeral" sx={{ minWidth: 26, textAlign: 'center', fontWeight: 600 }}>
                            {show(value)}
                          </Typography>
                          <IconButton
                            size="small"
                            aria-label={`More in the ${label.toLowerCase()}`}
                            disabled={value >= remaining}
                            onClick={() => move(where === 'chilled' ? chilled + STEP : chilled - STEP)}
                          >
                            <AddIcon fontSize="small" />
                          </IconButton>
                        </Stack>
                      );
                    })}
                  </Stack>
                ) : null}
              </Box>
            );
          })}

          {subjects.length === 0 ? (
            <Alert severity="info">This {isSnack ? 'snack' : 'meal'} has already been recorded.</Alert>
          ) : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={review.isPending}>Cancel</Button>
        <Button
          variant="contained"
          onClick={() => void confirm()}
          disabled={review.isPending || subjects.length === 0}
        >
          {isSnack ? 'Record snack' : 'Record meal'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
