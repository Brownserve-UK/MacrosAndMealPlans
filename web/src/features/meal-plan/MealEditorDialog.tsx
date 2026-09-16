import AddIcon from '@mui/icons-material/AddOutlined';
import CheckIcon from '@mui/icons-material/CheckOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { Amount, MealSlot, PlannerMeal } from '../../api/client';
import { ApiError } from '../../api/client';
import {
  useCreateMealPlanEntry,
  useHouseholdSettings,
  useHouseholdSlotAttendance,
  useMembers,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { parseIsoDate } from './date';
import {
  amountForMeal,
  forecastServings,
  initialFoods,
  initialMaking,
  MealFoodFields,
} from './MealFoodFields';
import { memberBlockedReason } from './mealAttendance';
import type { PlannableDish } from './UseItUp';

type EditorMode = 'member' | 'household';

function equalShare(amount: Amount, dinerCount: number): Amount {
  return { ...amount, value: amount.value / Math.max(dinerCount, 1) };
}

export function MealEditorDialog({
  open,
  onClose,
  date,
  slot,
  meal,
  mode,
  startWith,
}: {
  open: boolean;
  onClose: () => void;
  date: string;
  slot: MealSlot;
  meal: PlannerMeal | null;
  mode: EditorMode;
  startWith?: PlannableDish | null;
}) {
  const { principal } = useAuth();
  const household = mode === 'household';
  const members = useMembers({ include_archived: false, per_page: 200 });
  const mealTimes = useHouseholdSettings();
  const create = useCreateMealPlanEntry();
  const update = useUpdateMealPlanEntry();
  const [plannedTimeOverride, setPlannedTimeOverride] = useState<string | null>(meal ? meal.planned_time ?? '' : null);
  const mealNoun = slot === 'snacks' ? 'snack' : 'meal';
  const [selectedMembers, setSelectedMembers] = useState<string[]>(
    household
      ? meal?.people.map((person) => person.member_id) ?? []
      : principal?.member_id
        ? [principal.member_id]
        : [],
  );
  const [guestCount, setGuestCount] = useState(meal?.guest_groups.reduce((sum, group) => sum + group.count, 0) ?? 0);
  const [foods, setFoods] = useState(() => initialFoods(meal, startWith));
  const diners = household ? selectedMembers.length + guestCount : 1;
  const [making, setMaking] = useState<number | null>(() => initialMaking(meal, diners));
  const [error, setError] = useState<string | null>(null);
  const busy = create.isPending || update.isPending;
  const attendance = useHouseholdSlotAttendance(household ? date : '', household ? slot : '', meal?.id);
  const visibleMembers = useMemo(() => members.data?.items ?? [], [members.data]);
  const forecast = forecastServings(making, diners);
  const plannedTime = plannedTimeOverride ?? (slot === 'snacks' ? '' : mealTimes.data?.[slot] ?? '');
  const slotLabel = slot === 'snacks' ? 'snack' : slot;
  const dateLabel = parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });

  function toggleMember(memberId: string) {
    setSelectedMembers((current) => current.includes(memberId)
      ? current.filter((id) => id !== memberId)
      : [...current, memberId]);
  }

  async function save() {
    if (foods.length === 0) {
      setError('Add at least one food.');
      return;
    }
    if (foods.some((food) => food.itemKind !== 'recipe' && food.itemKind !== 'dish'
      && (!Number.isFinite(food.amount.value) || food.amount.value <= 0))) {
      setError('Every food needs an amount greater than zero.');
      return;
    }
    if (household && diners === 0) {
      setError(`Choose who is eating this ${mealNoun}.`);
      return;
    }
    setError(null);

    const components = foods.map((food) => ({
      id: food.componentId,
      ...(food.itemKind === 'product'
        ? { item_kind: 'product' as const, product_id: food.itemId }
        : food.itemKind === 'dish'
          ? { item_kind: 'dish' as const, dish_recipe_id: food.itemId }
          : food.itemKind === 'ingredient'
            ? { item_kind: 'ingredient' as const, ingredient_id: food.itemId }
            : food.itemKind === 'prepared_meal'
              ? { item_kind: 'prepared_meal' as const, prepared_meal_id: food.itemId }
              : { item_kind: 'recipe' as const, recipe_id: food.itemId }),
      amount: amountForMeal(food, forecast),
    }));
    const participants = household
      ? selectedMembers.map((memberId) => ({ member_id: memberId, allocations: [] }))
      : undefined;
    const guestAllocations = household && guestCount > 0
      ? foods.map((food) => ({
          component_id: food.componentId,
          amount: food.itemKind === 'recipe'
            ? { kind: 'servings' as const, value: 1 }
            : equalShare(food.amount, diners),
        }))
      : [];

    try {
      if (meal) {
        await update.mutateAsync({
          id: meal.id,
          revision: meal.revision,
          body: {
            planned_on: date,
            slot,
            planned_time: plannedTime || null,
            components,
            ...(household ? { participants, guest_count: guestCount, guest_allocations: guestAllocations } : {}),
          },
        });
      } else {
        await create.mutateAsync({
          planned_on: date,
          slot,
          planned_time: plannedTime || null,
          household,
          components,
          participants,
          guest_count: household ? guestCount : 0,
          guest_allocations: guestAllocations,
        });
      }
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not save this meal.');
    }
  }

  return (
    <FormDialog open={open} onClose={busy ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">{meal ? `Edit ${slotLabel}` : `Plan ${slotLabel}`}</Typography>
        <Typography component="span" variant="body2" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>{dateLabel}</Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}
          <TextField
            label={slot === 'snacks' ? 'Time (optional)' : 'Time'}
            type="time"
            value={plannedTime}
            onChange={(event) => setPlannedTimeOverride(event.target.value)}
            slotProps={{ inputLabel: { shrink: true } }}
            sx={{ width: { xs: '100%', sm: 180 } }}
          />
          <Box>
            <Typography variant="h3" sx={{ mb: 1 }}>Food</Typography>
            <MealFoodFields
              foods={foods}
              setFoods={setFoods}
              dinerCount={diners}
              making={making}
              setMaking={setMaking}
              mealNoun={mealNoun}
            />
          </Box>
          {household ? (
            <Box>
              <Stack direction="row" spacing={1} sx={{ alignItems: 'baseline', mb: 1 }}>
                <Typography variant="h3">Eating</Typography>
                {attendance.isFetching ? <Typography variant="caption" color="text.secondary">Checking who's free…</Typography> : null}
              </Stack>
              <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1, alignItems: 'center' }}>
                {visibleMembers.map((member) => {
                  const blocked = memberBlockedReason(attendance.data ?? [], member.id, meal?.people.map((person) => person.member_id));
                  const picked = selectedMembers.includes(member.id);
                  const chip = (
                    <Chip
                      label={member.display_name}
                      icon={picked ? <CheckIcon /> : undefined}
                      aria-disabled={blocked ? true : undefined}
                      aria-pressed={blocked ? undefined : picked}
                      onClick={blocked ? undefined : () => toggleMember(member.id)}
                      disabled={attendance.isFetching}
                      variant="outlined"
                      sx={{
                        borderRadius: 999,
                        height: 36,
                        px: 0.5,
                        ...(picked ? { bgcolor: 'action.selected', borderColor: 'primary.main', color: 'primary.main' } : {}),
                        ...(blocked ? { borderStyle: 'dashed', color: 'text.disabled', cursor: 'default' } : {}),
                      }}
                    />
                  );
                  return blocked
                    ? <Tooltip key={member.id} title={blocked}><span>{chip}</span></Tooltip>
                    : <Box key={member.id} component="span">{chip}</Box>;
                })}
                <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2, pl: 1.5, height: 36 }}>
                  <Typography variant="body2" color="text.secondary">Guests</Typography>
                  <IconButton size="small" aria-label="Remove guest" disabled={guestCount === 0} onClick={() => setGuestCount(Math.max(0, guestCount - 1))}><RemoveIcon fontSize="small" /></IconButton>
                  <Typography className="numeral" sx={{ minWidth: 16, textAlign: 'center' }}>{guestCount}</Typography>
                  <IconButton size="small" aria-label="Add guest" onClick={() => setGuestCount(guestCount + 1)}><AddIcon fontSize="small" /></IconButton>
                </Stack>
              </Stack>
            </Box>
          ) : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={busy}>Cancel</Button>
        <Button variant="contained" onClick={() => void save()} disabled={busy || foods.length === 0 || (household && diners === 0)}>
          {meal ? 'Save changes' : `Plan ${mealNoun}`}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
