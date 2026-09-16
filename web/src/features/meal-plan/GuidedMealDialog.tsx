import AddIcon from '@mui/icons-material/AddOutlined';
import ArrowBackIcon from '@mui/icons-material/ArrowBackIosNewOutlined';
import CheckIcon from '@mui/icons-material/CheckOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlineOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
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
  MealFoodFields,
} from './MealFoodFields';
import { memberBlockedReason } from './mealAttendance';
import { labelForSlot } from './slots';

type Step = 'attendance' | 'food';

function equalShare(amount: Amount, dinerCount: number): Amount {
  return { ...amount, value: amount.value / Math.max(dinerCount, 1) };
}

function OptionCard({
  title,
  caption,
  icon,
  selected,
  disabled,
  onClick,
}: {
  title: string;
  caption?: string;
  icon?: ReactNode;
  selected: boolean;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <ButtonBase
      onClick={onClick}
      disabled={disabled}
      aria-pressed={disabled ? undefined : selected}
      aria-disabled={disabled ? true : undefined}
      sx={{ display: 'block', width: '100%', textAlign: 'left', borderRadius: '14px' }}
    >
      <Paper
        elevation={0}
        sx={{
          width: '100%',
          p: 2,
          borderColor: selected ? 'primary.main' : 'divider',
          ...(disabled ? { borderStyle: 'dashed', color: 'text.disabled' } : {}),
        }}
      >
        <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
          <Box sx={{ width: 34, height: 34, borderRadius: '10px', bgcolor: 'action.hover', display: 'grid', placeItems: 'center', color: disabled ? 'text.disabled' : 'text.secondary' }}>
            {selected ? <CheckIcon /> : icon}
          </Box>
          <Stack spacing={0.5}>
            <Typography variant="body1" sx={{ fontWeight: 500 }}>{title}</Typography>
            {caption ? <Typography variant="caption" color={disabled ? 'text.disabled' : 'text.secondary'}>{caption}</Typography> : null}
          </Stack>
        </Stack>
      </Paper>
    </ButtonBase>
  );
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
  const memberId = principal?.member_id ?? null;
  const members = useMembers({ include_archived: false, per_page: 200 });
  const attendance = useHouseholdSlotAttendance(manager ? date : '', manager ? slot : '');
  const mealTimes = useHouseholdSettings();
  const create = useCreateMealPlanEntry();
  const [step, setStep] = useState<Step>(manager ? 'attendance' : 'food');
  const [direction, setDirection] = useState<'forward' | 'back'>('forward');
  const [transitioning, setTransitioning] = useState(false);
  const transitionTimer = useRef<number | null>(null);
  const [selectedMembers, setSelectedMembers] = useState<string[]>(memberId ? [memberId] : []);
  const [guestCount, setGuestCount] = useState(0);
  const [plannedTimeOverride, setPlannedTimeOverride] = useState<string | null>(null);
  const [editingTime, setEditingTime] = useState(false);
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
      if (manager) go('attendance', 'back');
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

  const headerTime = editingTime ? (
    <TextField
      autoFocus
      aria-label={slot === 'snacks' ? 'Time (optional)' : 'Time'}
      type="time"
      size="small"
      value={plannedTime}
      onChange={(event) => setPlannedTimeOverride(event.target.value)}
      onBlur={() => setEditingTime(false)}
      slotProps={{ htmlInput: { style: { paddingTop: 6, paddingBottom: 6 } } }}
      sx={{ width: 128 }}
    />
  ) : (
    <Chip
      size="small"
      icon={<EditIcon />}
      label={`${labelForSlot(slot)} · ${plannedTime || 'Add time'}`}
      onClick={() => setEditingTime(true)}
      sx={{ borderRadius: 999, bgcolor: 'action.hover', border: 0 }}
    />
  );

  return (
    <Dialog
      open={open}
      onClose={create.isPending ? undefined : onClose}
      aria-labelledby="guided-meal-title"
      fullWidth
      maxWidth="sm"
      slotProps={{ paper: { sx: { minHeight: { xs: '80vh', sm: 600 } } } }}
    >
      <Stack sx={{ minHeight: 'inherit' }}>
        <Box sx={{ display: 'grid', gridTemplateColumns: '40px minmax(0, 1fr) 40px', alignItems: 'center', gap: 1, px: 2, pt: 2 }}>
          {manager ? (
            <IconButton
              aria-label="Back"
              onClick={() => step === 'food' && go('attendance', 'back')}
              sx={{ visibility: step === 'attendance' ? 'hidden' : 'visible' }}
            >
              <ArrowBackIcon />
            </IconButton>
          ) : <Box sx={{ width: 40 }} />}
          {manager ? (
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
        <Box sx={{ display: 'flex', justifyContent: 'center', mt: -0.5, pb: 1 }}>
          {headerTime}
        </Box>

        <Box sx={{ flex: 1, px: 3, py: { xs: 2, sm: 3 }, overflow: 'auto' }}>
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
            <Typography id="guided-meal-title" variant="h1">
              {step === 'attendance' ? "Who's eating?" : 'What are you eating?'}
            </Typography>
            {error ? <Alert severity="error">{error}</Alert> : null}

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
                <Button variant="contained" size="large" onClick={() => go('food')} disabled={diners === 0 || attendance.isFetching}>
                  Continue
                </Button>
              </Stack>
            ) : (
              <Stack spacing={3}>
                <MealFoodFields
                  foods={foods}
                  setFoods={setFoods}
                  dinerCount={diners}
                  making={making}
                  setMaking={setMaking}
                  mealNoun={mealNoun}
                />
                <Button variant="contained" size="large" onClick={() => void save()} disabled={create.isPending || foods.length === 0 || diners === 0}>
                  {create.isPending ? 'Planning…' : `Plan ${mealNoun}`}
                </Button>
              </Stack>
            )}
          </Stack>
        </Box>
      </Stack>
    </Dialog>
  );
}
