import AddIcon from '@mui/icons-material/AddOutlined';
import ArrowBackIcon from '@mui/icons-material/ArrowBackIosNewOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlineOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { Amount, MealSlot } from '../../api/client';
import { ApiError } from '../../api/client';
import {
  useCreateMealPlanEntry,
  useHouseholdSettings,
  useHouseholdSlotAttendance,
  useMembers,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import {
  amountForMeal,
  forecastServings,
  initialFoods,
  FoodList,
  FoodSearchField,
} from './MealFoodFields';
import { memberBlockedReason } from './mealAttendance';
import { MealTimeChip, OptionCard } from './MealDialogParts';

type Step = 'attendance' | 'food';

function equalShare(amount: Amount, dinerCount: number): Amount {
  return { ...amount, value: amount.value / Math.max(dinerCount, 1) };
}

export function GuidedMealDialog({
  open,
  onClose,
  date,
  slot,
}: {
  open: boolean;
  onClose: () => void;
  date: string;
  slot: MealSlot;
}) {
  const { principal } = useAuth();
  const manager = principal?.permissions.includes('household:write') ?? false;
  const singleAttendeeOnly = slot === 'snacks';
  const showAttendanceStep = manager && !singleAttendeeOnly;
  const memberId = principal?.member_id ?? null;
  const members = useMembers({ include_archived: false, per_page: 200 });
  const attendance = useHouseholdSlotAttendance(showAttendanceStep ? date : '', showAttendanceStep ? slot : '');
  const mealTimes = useHouseholdSettings();
  const create = useCreateMealPlanEntry();
  const [step, setStep] = useState<Step>(showAttendanceStep ? 'attendance' : 'food');
  const [direction, setDirection] = useState<'forward' | 'back'>('forward');
  const [transitioning, setTransitioning] = useState(false);
  const transitionTimer = useRef<number | null>(null);
  const [selectedMembers, setSelectedMembers] = useState<string[]>(memberId ? [memberId] : []);
  const [guestCount, setGuestCount] = useState(0);
  const [plannedTimeOverride, setPlannedTimeOverride] = useState<string | null>(null);
  const [foods, setFoods] = useState(() => initialFoods(null));
  const [making, setMaking] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const mealNoun = slot === 'snacks' ? 'snack' : 'meal';
  const plannedTime = plannedTimeOverride ?? (slot === 'snacks' ? '' : mealTimes.data?.[slot] ?? '');
  const otherMembers = useMemo(
    () => (members.data?.items ?? []).filter((member) => member.id !== memberId),
    [memberId, members.data],
  );
  const currentMemberBlocked = memberId ? memberBlockedReason(attendance.data ?? [], memberId) : null;
  const selectedAttendees = manager
    ? selectedMembers.filter((selectedMemberId) => !memberBlockedReason(attendance.data ?? [], selectedMemberId))
    : selectedMembers;
  const diners = selectedAttendees.length + guestCount;
  const forecast = forecastServings(making, diners);

  useEffect(() => () => {
    if (transitionTimer.current != null) window.clearTimeout(transitionTimer.current);
  }, []);

  function go(next: Step, nextDirection: 'forward' | 'back' = 'forward') {
    setDirection(nextDirection);
    setError(null);
    setTransitioning(true);
    transitionTimer.current = window.setTimeout(() => {
      setStep(next);
      setTransitioning(false);
    }, 150);
  }

  function toggleMember(nextMemberId: string) {
    setSelectedMembers((current) => current.includes(nextMemberId)
      ? current.filter((id) => id !== nextMemberId)
      : [...current, nextMemberId]);
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
    if (diners === 0) {
      setError(`Choose who is eating this ${mealNoun}.`);
      if (showAttendanceStep) go('attendance', 'back');
      return;
    }
    setError(null);
    const household = selectedAttendees.length !== 1 || guestCount > 0;
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
    const guestAllocations = household && guestCount > 0
      ? foods.map((food) => ({
          component_id: food.componentId,
          amount: food.itemKind === 'recipe'
            ? { kind: 'servings' as const, value: 1 }
            : equalShare(food.amount, diners),
        }))
      : [];

    try {
      await create.mutateAsync({
        planned_on: date,
        slot,
        planned_time: plannedTime || null,
        household,
        member_id: household ? null : selectedAttendees[0],
        components,
        participants: household
          ? selectedAttendees.map((selectedMemberId) => ({ member_id: selectedMemberId, allocations: [] }))
          : undefined,
        guest_count: household ? guestCount : 0,
        guest_allocations: guestAllocations,
      });
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not save this meal.');
    }
  }

  return (
    <Dialog
      open={open}
      onClose={create.isPending ? undefined : onClose}
      aria-labelledby="guided-meal-title"
      fullWidth
      maxWidth="sm"
      slotProps={{ paper: { sx: { overflow: 'hidden' } } }}
    >
      <Stack sx={{ minHeight: { xs: '70vh', sm: 520 }, maxHeight: { xs: '90vh', sm: '85vh' }, overflow: 'hidden' }}>
        <Box sx={{ display: 'grid', gridTemplateColumns: '40px minmax(0, 1fr) 40px', alignItems: 'center', gap: 1, px: 2, pt: 2, flexShrink: 0 }}>
          {showAttendanceStep ? (
            <IconButton
              aria-label="Back"
              onClick={() => step === 'food' && go('attendance', 'back')}
              sx={{ visibility: step === 'attendance' ? 'hidden' : 'visible' }}
            >
              <ArrowBackIcon />
            </IconButton>
          ) : <Box sx={{ width: 40 }} />}
          {showAttendanceStep ? (
            <Stack direction="row" spacing={1} aria-label="Progress" sx={{ justifySelf: 'center' }}>
              {(['attendance', 'food'] as const).map((item, index) => {
                const stepIndex = step === 'attendance' ? 0 : 1;
                return (
                  <Box
                    key={item}
                    sx={{
                      width: index === stepIndex ? 10 : 8,
                      height: index === stepIndex ? 10 : 8,
                      borderRadius: '50%',
                      bgcolor: index < stepIndex ? 'primary.dark' : index === stepIndex ? 'primary.main' : 'action.disabledBackground',
                      transition: 'width 150ms, height 150ms, background-color 150ms',
                    }}
                  />
                );
              })}
            </Stack>
          ) : <Box />}
          <IconButton aria-label="Close" onClick={onClose} disabled={create.isPending}><CloseIcon /></IconButton>
        </Box>
        <Box sx={{ display: 'flex', justifyContent: 'center', mt: -0.5, pb: 1, flexShrink: 0 }}>
          <MealTimeChip slot={slot} time={plannedTime} onChange={setPlannedTimeOverride} />
        </Box>

        <Box sx={{ px: 3, pt: { xs: 1, sm: 2 }, pb: 2, flexShrink: 0 }}>
          <Stack spacing={2} sx={{ width: '100%', maxWidth: 480, mx: 'auto' }}>
            <Typography id="guided-meal-title" variant="h1">
              {step === 'attendance' ? "Who's eating?" : 'What are you eating?'}
            </Typography>
            {error ? <Alert severity="error">{error}</Alert> : null}
            {step === 'food' ? (
              <FoodSearchField foods={foods} setFoods={setFoods} dinerCount={diners} making={making} />
            ) : null}
          </Stack>
        </Box>

        <Box sx={{ flex: 1, minHeight: 0, px: 3, py: { xs: 2, sm: 3 }, overflow: 'auto' }}>
          <Stack
            key={step}
            spacing={3}
            sx={{
              width: '100%',
              maxWidth: 480,
              mx: 'auto',
              pointerEvents: transitioning ? 'none' : 'auto',
              animation: `${transitioning ? direction === 'forward' ? 'stepExitForward' : 'stepExitBack' : direction === 'forward' ? 'stepEnterForward' : 'stepEnterBack'} 150ms ease-in-out`,
              '@keyframes stepEnterForward': { from: { opacity: 0, transform: 'translateX(32px)' }, to: { opacity: 1, transform: 'translateX(0)' } },
              '@keyframes stepEnterBack': { from: { opacity: 0, transform: 'translateX(-32px)' }, to: { opacity: 1, transform: 'translateX(0)' } },
              '@keyframes stepExitForward': { from: { opacity: 1, transform: 'translateX(0)' }, to: { opacity: 0, transform: 'translateX(-32px)' } },
              '@keyframes stepExitBack': { from: { opacity: 1, transform: 'translateX(0)' }, to: { opacity: 0, transform: 'translateX(32px)' } },
            }}
          >
            {step === 'attendance' ? (
              <Stack spacing={2}>
                {memberId ? (
                  currentMemberBlocked ? (
                    <Tooltip title={currentMemberBlocked}>
                      <span>
                        <OptionCard
                          title="Just me"
                          caption={currentMemberBlocked}
                          icon={<PersonIcon />}
                          selected={selectedAttendees.includes(memberId)}
                          disabled
                          onClick={() => toggleMember(memberId)}
                        />
                      </span>
                    </Tooltip>
                  ) : (
                    <OptionCard
                      title="Just me"
                      caption="Plan this for yourself"
                      icon={<PersonIcon />}
                      selected={selectedAttendees.includes(memberId)}
                      disabled={attendance.isFetching}
                      onClick={() => toggleMember(memberId)}
                    />
                  )
                ) : null}
                {otherMembers.map((member) => {
                  const blocked = memberBlockedReason(attendance.data ?? [], member.id);
                  const card = (
                    <OptionCard
                      title={member.display_name}
                      caption={blocked ?? 'Household member'}
                      icon={<PersonIcon />}
                      selected={selectedAttendees.includes(member.id)}
                      disabled={attendance.isFetching || Boolean(blocked)}
                      onClick={() => toggleMember(member.id)}
                    />
                  );
                  return blocked
                    ? <Tooltip key={member.id} title={blocked}><span>{card}</span></Tooltip>
                    : <Box key={member.id}>{card}</Box>;
                })}
                <Stack direction="row" spacing={1} sx={{ alignItems: 'center', justifyContent: 'space-between', px: 1 }}>
                  <Typography variant="body1" sx={{ fontWeight: 500 }}>Guests</Typography>
                  <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
                    <IconButton aria-label="Remove guest" disabled={guestCount === 0} onClick={() => setGuestCount((count) => Math.max(0, count - 1))}><RemoveIcon /></IconButton>
                    <Typography className="numeral" sx={{ minWidth: 24, textAlign: 'center' }}>{guestCount}</Typography>
                    <IconButton aria-label="Add guest" onClick={() => setGuestCount((count) => count + 1)}><AddIcon /></IconButton>
                  </Stack>
                </Stack>
                {attendance.isFetching ? <Typography variant="caption" color="text.secondary">Checking who's free…</Typography> : null}
              </Stack>
            ) : (
              <FoodList
                foods={foods}
                setFoods={setFoods}
                dinerCount={diners}
                making={making}
                setMaking={setMaking}
                mealNoun={mealNoun}
              />
            )}
          </Stack>
        </Box>

        <Box sx={{ px: 3, pb: { xs: 2, sm: 3 }, pt: 1, flexShrink: 0 }}>
          <Stack sx={{ width: '100%', maxWidth: 480, mx: 'auto' }}>
            {step === 'attendance' ? (
              <Button variant="contained" size="large" onClick={() => go('food')} disabled={diners === 0 || attendance.isFetching}>
                Continue
              </Button>
            ) : (
              <Button variant="contained" size="large" onClick={() => void save()} disabled={create.isPending || foods.length === 0 || diners === 0}>
                {create.isPending ? 'Planning…' : `Plan ${mealNoun}`}
              </Button>
            )}
          </Stack>
        </Box>
      </Stack>
    </Dialog>
  );
}
