import AddIcon from '@mui/icons-material/AddOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlineOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
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
  FoodList,
  FoodSearchField,
} from './MealFoodFields';
import { memberBlockedReason } from './mealAttendance';
import { MealTimeChip, OptionCard } from './MealDialogParts';
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
  const singleAttendeeOnly = slot === 'snacks';
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
    if (singleAttendeeOnly) {
      setGuestCount(0);
      setSelectedMembers((current) => (current.includes(memberId) ? [] : [memberId]));
      return;
    }
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
    <FormDialog open={open} onClose={busy ? undefined : onClose} fullWidth maxWidth="sm" slotProps={{ paper: { sx: { overflow: 'hidden' } } }}>
      <Stack sx={{ minHeight: { xs: '70vh', sm: 520 }, maxHeight: { xs: '90vh', sm: '85vh' }, overflow: 'hidden' }}>
        <Box sx={{ display: 'grid', gridTemplateColumns: '40px minmax(0, 1fr) 40px', alignItems: 'center', gap: 1, px: 2, pt: 2, flexShrink: 0 }}>
          <Box sx={{ width: 40 }} />
          <Box />
          <IconButton aria-label="Close" onClick={onClose} disabled={busy}><CloseIcon /></IconButton>
        </Box>
        <Box sx={{ display: 'flex', justifyContent: 'center', mt: -0.5, pb: 1, flexShrink: 0 }}>
          <MealTimeChip slot={slot} time={plannedTime} onChange={setPlannedTimeOverride} />
        </Box>

        <Box sx={{ px: 3, pt: { xs: 1, sm: 2 }, pb: 2, flexShrink: 0 }}>
          <Stack spacing={2} sx={{ width: '100%', maxWidth: 480, mx: 'auto' }}>
            <Box sx={{ textAlign: 'center' }}>
              <Typography variant="h1">{`${meal ? 'Edit' : 'Plan'} ${slotLabel}`}</Typography>
              <Typography variant="body2" color="text.secondary" className="numeral" sx={{ mt: 0.5 }}>{dateLabel}</Typography>
            </Box>
            {error ? <Alert severity="error">{error}</Alert> : null}
            <FoodSearchField foods={foods} setFoods={setFoods} dinerCount={diners} making={making} />
          </Stack>
        </Box>

        <Box sx={{ flex: 1, minHeight: 0, px: 3, py: { xs: 2, sm: 3 }, overflow: 'auto' }}>
          <Stack spacing={3} sx={{ width: '100%', maxWidth: 480, mx: 'auto' }}>
            {household ? (
              <Stack spacing={2}>
                <Typography variant="overline" color="text.secondary">Eating</Typography>
                {singleAttendeeOnly ? (
                  <Typography variant="caption" color="text.secondary">Snacks are planned for one person at a time.</Typography>
                ) : null}
                {visibleMembers.map((member) => {
                  const blocked = memberBlockedReason(attendance.data ?? [], member.id, meal?.people.map((person) => person.member_id));
                  const card = (
                    <OptionCard
                      title={member.display_name}
                      caption={blocked ?? 'Household member'}
                      icon={<PersonIcon />}
                      selected={selectedMembers.includes(member.id)}
                      disabled={attendance.isFetching || Boolean(blocked)}
                      onClick={() => toggleMember(member.id)}
                    />
                  );
                  return blocked
                    ? <Tooltip key={member.id} title={blocked}><span>{card}</span></Tooltip>
                    : <Box key={member.id}>{card}</Box>;
                })}
                {singleAttendeeOnly ? null : (
                  <Stack direction="row" spacing={1} sx={{ alignItems: 'center', justifyContent: 'space-between', px: 1 }}>
                    <Typography variant="body1" sx={{ fontWeight: 500 }}>Guests</Typography>
                    <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
                      <IconButton size="small" aria-label="Remove guest" disabled={guestCount === 0} onClick={() => setGuestCount(Math.max(0, guestCount - 1))}><RemoveIcon fontSize="small" /></IconButton>
                      <Typography className="numeral" sx={{ minWidth: 24, textAlign: 'center' }}>{guestCount}</Typography>
                      <IconButton size="small" aria-label="Add guest" onClick={() => setGuestCount(guestCount + 1)}><AddIcon fontSize="small" /></IconButton>
                    </Stack>
                  </Stack>
                )}
                {attendance.isFetching ? <Typography variant="caption" color="text.secondary">Checking who's free…</Typography> : null}
              </Stack>
            ) : null}

            <Stack spacing={1}>
              <Typography variant="overline" color="text.secondary">Food</Typography>
              <FoodList
                foods={foods}
                setFoods={setFoods}
                dinerCount={diners}
                making={making}
                setMaking={setMaking}
                mealNoun={mealNoun}
              />
            </Stack>
          </Stack>
        </Box>

        <Box sx={{ px: 3, pb: { xs: 2, sm: 3 }, pt: 1, flexShrink: 0 }}>
          <Stack sx={{ width: '100%', maxWidth: 480, mx: 'auto' }}>
            <Button variant="contained" size="large" onClick={() => void save()} disabled={busy || foods.length === 0 || (household && diners === 0)}>
              {busy ? 'Saving…' : meal ? 'Save changes' : `Plan ${mealNoun}`}
            </Button>
          </Stack>
        </Box>
      </Stack>
    </FormDialog>
  );
}
