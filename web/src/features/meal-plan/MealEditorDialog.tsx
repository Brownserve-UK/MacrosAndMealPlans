import AddIcon from '@mui/icons-material/AddOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Checkbox from '@mui/material/Checkbox';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import Step from '@mui/material/Step';
import StepLabel from '@mui/material/StepLabel';
import Stepper from '@mui/material/Stepper';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { Amount, MealSlot, PlannerMeal, Product, RecipeSummary, Unit } from '../../api/client';
import { ApiError } from '../../api/client';
import {
  useCreateMealPlanEntry,
  useHouseholdSlotAttendance,
  useMealTimes,
  useMembers,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { displayUnit } from '../../components/UnitSelect';
import { parseIsoDate } from './date';
import { FoodSearch, type FoodChoice } from './FoodSearch';

type EditorMode = 'member' | 'household';

type FoodDraft = {
  componentId: string;
  itemKind: 'product' | 'recipe';
  itemId: string;
  name: string;
  amount: Amount;
};

type PortionValues = Record<string, string>;

const UNITS: Unit[] = ['mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch'];

function amountValue(amount: Amount) {
  return String(amount.value);
}

function withAmountValue(amount: Amount, raw: string): Amount {
  return { ...amount, value: Number(raw) || 0 };
}

function equalShare(amount: Amount, dinerCount: number): Amount {
  return { ...amount, value: amount.value / Math.max(dinerCount, 1) };
}

function allocationKey(componentId: string, subjectId: string) {
  return `${componentId}:${subjectId}`;
}

function initialFoods(meal: PlannerMeal | null): FoodDraft[] {
  return (meal?.foods ?? []).map((food) => ({
    componentId: food.id,
    itemKind: food.item_kind,
    itemId: food.item_kind === 'product' ? food.product_id : food.recipe_id,
    name: food.item_name,
    amount: food.amount,
  }));
}

function initialPortions(meal: PlannerMeal | null): PortionValues {
  if (!meal) return {};
  const values: PortionValues = {};
  for (const person of meal.people) {
    for (const allocation of person.allocations) {
      values[allocationKey(allocation.component_id, person.member_id)] = String(allocation.allocated.value);
    }
  }
  for (const group of meal.guest_groups) {
    for (const allocation of group.allocations) {
      values[allocationKey(allocation.component_id, 'guest')] = String(allocation.allocated.value);
    }
  }
  return values;
}

export function MealEditorDialog({
  open,
  onClose,
  date,
  slot,
  meal,
  mode,
}: {
  open: boolean;
  onClose: () => void;
  date: string;
  slot: MealSlot;
  meal: PlannerMeal | null;
  mode: EditorMode;
}) {
  const { principal } = useAuth();
  const household = mode === 'household';
  const members = useMembers({ include_archived: false, per_page: 200 });
  const mealTimes = useMealTimes();
  const create = useCreateMealPlanEntry();
  const update = useUpdateMealPlanEntry();
  const [plannedTimeOverride, setPlannedTimeOverride] = useState<string | null>(
    meal ? meal.planned_time ?? '' : null,
  );
  const [step, setStep] = useState(0);
  const mealNoun = slot === 'snacks' ? 'snack' : 'meal';
  const [selectedMembers, setSelectedMembers] = useState<string[]>(
    household
      ? meal?.people.map((person) => person.member_id) ?? []
      : principal?.member_id
        ? [principal.member_id]
        : [],
  );
  const [guestCount, setGuestCount] = useState(meal?.guest_groups.reduce((sum, group) => sum + group.count, 0) ?? 0);
  const [foods, setFoods] = useState<FoodDraft[]>(() => initialFoods(meal));
  const [customPortions, setCustomPortions] = useState(() => meal?.portioning === 'custom');
  const [portions, setPortions] = useState<PortionValues>(() => initialPortions(meal));
  const [error, setError] = useState<string | null>(null);
  const busy = create.isPending || update.isPending;

  const attendance = useHouseholdSlotAttendance(
    household ? date : '',
    household ? slot : '',
    meal?.id,
  );
  const attendanceByMember = useMemo(() => {
    const map = new Map<string, { state: string; claimedTime: string | null }>();
    for (const row of attendance.data ?? []) {
      map.set(row.member_id, { state: row.attendance, claimedTime: row.claimed_time ?? null });
    }
    return map;
  }, [attendance.data]);

  const visibleMembers = useMemo(() => members.data?.items ?? [], [members.data]);
  const diners = household ? selectedMembers.length + guestCount : 1;
  const plannedTime = plannedTimeOverride
    ?? (slot === 'snacks' ? '' : mealTimes.data?.[slot] ?? '');
  const slotLabel = slot === 'snacks' ? 'snack' : slot;
  const dateLabel = parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });

  function memberBlockedReason(memberId: string): string | null {
    if (meal?.people.some((person) => person.member_id === memberId)) return null;
    const row = attendanceByMember.get(memberId);
    if (!row) return null;
    if (row.state === 'self_catering') return 'Has their own plan';
    if (row.state === 'opted_out') return 'Opted out';
    if (row.state === 'participating') {
      return row.claimedTime ? `Already eating at ${row.claimedTime}` : 'Already in another meal';
    }
    return null;
  }

  function addProduct(next: Product) {
    if (foods.some((food) => food.itemKind === 'product' && food.itemId === next.id)) return;
    const food: FoodDraft = {
      componentId: crypto.randomUUID(),
      itemKind: 'product',
      itemId: next.id,
      name: next.name,
      amount: {
        kind: 'measure',
        unit: next.nutrition.basis?.unit ?? next.package_quantity?.unit ?? 'g',
        value: next.nutrition.basis?.amount ?? 100,
      },
    };
    setFoods((current) => [...current, food]);
    if (customPortions) initialiseFoodPortions(food);
  }

  function addRecipe(next: RecipeSummary) {
    if (foods.some((food) => food.itemKind === 'recipe' && food.itemId === next.id)) return;
    const food: FoodDraft = {
      componentId: crypto.randomUUID(),
      itemKind: 'recipe',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'servings', value: 1 },
    };
    setFoods((current) => [...current, food]);
    if (customPortions) initialiseFoodPortions(food);
  }

  function addFood(choice: FoodChoice) {
    if (choice.kind === 'product') addProduct(choice.product);
    else addRecipe(choice.recipe);
  }

  function initialiseFoodPortions(food: FoodDraft) {
    const share = String(equalShare(food.amount, diners).value);
    setPortions((current) => {
      const next = { ...current };
      for (const memberId of selectedMembers) next[allocationKey(food.componentId, memberId)] = share;
      if (guestCount > 0) next[allocationKey(food.componentId, 'guest')] = share;
      return next;
    });
  }

  function setFoodAmount(componentId: string, amount: Amount) {
    setFoods((current) => current.map((food) => food.componentId === componentId ? { ...food, amount } : food));
  }

  function enableCustomPortions() {
    const next: PortionValues = {};
    for (const food of foods) {
      for (const memberId of selectedMembers) {
        next[allocationKey(food.componentId, memberId)] = String(equalShare(food.amount, diners).value);
      }
      if (guestCount > 0) next[allocationKey(food.componentId, 'guest')] = String(equalShare(food.amount, diners).value);
    }
    setPortions(next);
    setCustomPortions(true);
  }

  function toggleMember(memberId: string, checked: boolean) {
    setSelectedMembers((current) => checked ? [...current, memberId] : current.filter((id) => id !== memberId));
    if (!customPortions) return;
    setPortions((current) => {
      const next = { ...current };
      for (const food of foods) {
        const key = allocationKey(food.componentId, memberId);
        if (checked) next[key] = '0';
        else delete next[key];
      }
      return next;
    });
  }

  function setGuests(nextCount: number) {
    const safeCount = Math.max(0, nextCount);
    setGuestCount(safeCount);
    if (!customPortions || safeCount === 0 || guestCount > 0) return;
    setPortions((current) => {
      const next = { ...current };
      for (const food of foods) next[allocationKey(food.componentId, 'guest')] = '0';
      return next;
    });
  }

  function validateMeal() {
    if (foods.length === 0) {
      setError('Add at least one food.');
      return false;
    }
    if (foods.some((food) => !Number.isFinite(food.amount.value) || food.amount.value <= 0)) {
      setError('Every food needs an amount greater than zero.');
      return false;
    }
    setError(null);
    return true;
  }

  function continueToPeople() {
    if (!validateMeal()) return;
    setStep(1);
  }

  async function save() {
    if (!validateMeal()) {
      if (household) setStep(0);
      return;
    }
    if (household && selectedMembers.length + guestCount === 0) {
      setError('Choose at least one household member or guest.');
      return;
    }
    if (household && customPortions) {
      const invalid = foods.some((food) => {
        const memberTotal = selectedMembers.reduce((sum, memberId) => sum + (Number(portions[allocationKey(food.componentId, memberId)]) || 0), 0);
        const guestTotal = guestCount * (Number(portions[allocationKey(food.componentId, 'guest')]) || 0);
        return Math.abs(memberTotal + guestTotal - food.amount.value) > 0.0001;
      });
      if (invalid) {
        setError('Set amounts must add up to the total for each food.');
        return;
      }
    }

    const components = foods.map((food) => ({
      id: food.componentId,
      ...(food.itemKind === 'product'
        ? { item_kind: 'product' as const, product_id: food.itemId }
        : { item_kind: 'recipe' as const, recipe_id: food.itemId }),
      amount: food.amount,
    }));
    const participants = household
      ? selectedMembers.map((memberId) => ({
          member_id: memberId,
          allocations: customPortions
            ? foods.map((food) => ({
                component_id: food.componentId,
                amount: { ...food.amount, value: Number(portions[allocationKey(food.componentId, memberId)]) || 0 },
              }))
            : [],
        }))
      : undefined;
    const guestAllocations = household && guestCount > 0
      ? foods.map((food) => ({
          component_id: food.componentId,
          amount: customPortions
            ? { ...food.amount, value: Number(portions[allocationKey(food.componentId, 'guest')]) || 0 }
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
            portioning: household ? (customPortions ? 'custom' : 'equal') : undefined,
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
          portioning: household ? (customPortions ? 'custom' : 'equal') : undefined,
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

  const mealFields = (
    <Stack spacing={2.5}>
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
        <Stack spacing={1.5}>
          <FoodSearch
            onPick={addFood}
            excludeProductIds={foods.filter((food) => food.itemKind === 'product').map((food) => food.itemId)}
            excludeRecipeIds={foods.filter((food) => food.itemKind === 'recipe').map((food) => food.itemId)}
          />
          {foods.length === 0 ? (
            <Typography variant="body2" color="text.secondary">No food added yet.</Typography>
          ) : (
            <Stack spacing={1}>
              {foods.map((food) => (
                <Box key={food.componentId} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2, p: 1.5 }}>
                  <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5} sx={{ alignItems: { sm: 'center' } }}>
                    <Typography sx={{ flex: 1, minWidth: 0 }}>{food.name}</Typography>
                    <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                      <TextField
                        label="Amount"
                        type="number"
                        value={amountValue(food.amount)}
                        onChange={(event) => setFoodAmount(food.componentId, withAmountValue(food.amount, event.target.value))}
                        slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                        sx={{ width: 120 }}
                      />
                      {food.amount.kind === 'measure' ? (
                        <TextField
                          select
                          label="Unit"
                          value={food.amount.unit}
                          onChange={(event) => setFoodAmount(food.componentId, { kind: 'measure', value: food.amount.value, unit: event.target.value as Unit })}
                          sx={{ width: 110 }}
                        >
                          {UNITS.map((unit) => <MenuItem key={unit} value={unit}>{displayUnit(unit)}</MenuItem>)}
                        </TextField>
                      ) : (
                        <Typography color="text.secondary" sx={{ minWidth: 72 }}>
                          {food.amount.value === 1 ? 'serving' : 'servings'}
                        </Typography>
                      )}
                      <IconButton aria-label={`Remove ${food.name}`} onClick={() => setFoods((current) => current.filter((candidate) => candidate.componentId !== food.componentId))}>
                        <DeleteIcon />
                      </IconButton>
                    </Stack>
                  </Stack>
                </Box>
              ))}
            </Stack>
          )}
        </Stack>
      </Box>
    </Stack>
  );

  const peopleFields = (
    <Stack spacing={3}>
      <Box>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'baseline', mb: 1 }}>
          <Typography variant="h3">People</Typography>
          {attendance.isFetching ? <Typography variant="caption" color="text.secondary">Checking who's free…</Typography> : null}
        </Stack>
        <Stack spacing={0.75}>
          {visibleMembers.map((member) => {
            const blocked = memberBlockedReason(member.id);
            return (
              <Box key={member.id} component="label" sx={{ display: 'flex', alignItems: 'flex-start', border: '1px solid', borderColor: 'divider', borderRadius: 2, px: 1, py: 0.5 }}>
                <Checkbox
                  sx={{ mt: -0.25 }}
                  checked={selectedMembers.includes(member.id)}
                  disabled={Boolean(blocked) || attendance.isFetching}
                  onChange={(_, checked) => toggleMember(member.id, checked)}
                />
                <Box sx={{ py: 0.5 }}>
                  <Typography color={blocked ? 'text.disabled' : 'text.primary'}>{member.display_name}</Typography>
                  {blocked ? <Typography variant="caption" color="text.secondary">{blocked}</Typography> : null}
                </Box>
              </Box>
            );
          })}
        </Stack>
      </Box>

      <Box>
        <Typography variant="h3" sx={{ mb: 1 }}>Guests</Typography>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <IconButton aria-label="Remove guest" disabled={guestCount === 0} onClick={() => setGuests(guestCount - 1)}><RemoveIcon /></IconButton>
          <Typography className="numeral" sx={{ minWidth: 24, textAlign: 'center' }}>{guestCount}</Typography>
          <IconButton aria-label="Add guest" onClick={() => setGuests(guestCount + 1)}><AddIcon /></IconButton>
        </Stack>
      </Box>

      {diners > 0 && foods.length > 0 ? (
        <Box>
          <Typography variant="h3" sx={{ mb: 1 }}>Portions</Typography>
          <Stack direction="row" spacing={1} sx={{ mb: customPortions ? 2 : 0 }}>
            <Button variant={customPortions ? 'outlined' : 'contained'} onClick={() => setCustomPortions(false)}>Split equally</Button>
            <Button
              variant={customPortions ? 'contained' : 'outlined'}
              onClick={() => {
                if (!customPortions) enableCustomPortions();
              }}
            >
              Set amounts
            </Button>
          </Stack>
          {customPortions ? (
            <Stack spacing={1.5}>
              {foods.map((food) => (
                <Box key={food.componentId} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2, p: 1.5 }}>
                  <Typography sx={{ mb: 1, fontWeight: 600 }}>{food.name}</Typography>
                  <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.25} sx={{ flexWrap: 'wrap' }}>
                    {selectedMembers.map((memberId) => (
                      <TextField
                        key={memberId}
                        label={visibleMembers.find((member) => member.id === memberId)?.display_name ?? 'Member'}
                        type="number"
                        value={portions[allocationKey(food.componentId, memberId)] ?? ''}
                        onChange={(event) => setPortions((current) => ({ ...current, [allocationKey(food.componentId, memberId)]: event.target.value }))}
                        slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                        sx={{ width: 150 }}
                      />
                    ))}
                    {guestCount > 0 ? (
                      <TextField
                        label="Each guest"
                        type="number"
                        value={portions[allocationKey(food.componentId, 'guest')] ?? ''}
                        onChange={(event) => setPortions((current) => ({ ...current, [allocationKey(food.componentId, 'guest')]: event.target.value }))}
                        slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                        sx={{ width: 150 }}
                      />
                    ) : null}
                  </Stack>
                </Box>
              ))}
            </Stack>
          ) : null}
        </Box>
      ) : null}
    </Stack>
  );

  return (
    <FormDialog open={open} onClose={busy ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: household ? 1.5 : 1 }}>
        <Typography component="span" variant="h2">{meal ? `Edit ${slotLabel}` : `Plan ${slotLabel}`}</Typography>
        <Typography component="span" variant="body2" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>{dateLabel}</Typography>
      </DialogTitle>
      {household ? (
        <Box sx={{ px: 3, pb: 2 }}>
          <Stepper activeStep={step}>
            <Step><StepLabel>Meal</StepLabel></Step>
            <Step><StepLabel>People</StepLabel></Step>
          </Stepper>
        </Box>
      ) : null}
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}
          {household && step === 1 ? peopleFields : mealFields}
        </Stack>
      </DialogContent>
      <DialogActions>
        {household && step === 1 ? (
          <Button onClick={() => { setError(null); setStep(0); }} disabled={busy}>Back</Button>
        ) : (
          <Button onClick={onClose} disabled={busy}>Cancel</Button>
        )}
        {household && step === 0 ? (
          <Button variant="contained" onClick={continueToPeople} disabled={busy}>Continue</Button>
        ) : (
          <Button variant="contained" onClick={() => void save()} disabled={busy}>
            {meal ? 'Save changes' : `Plan ${mealNoun}`}
          </Button>
        )}
      </DialogActions>
    </FormDialog>
  );
}
