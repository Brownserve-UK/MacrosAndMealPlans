import AddIcon from '@mui/icons-material/AddOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Checkbox from '@mui/material/Checkbox';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import FormControlLabel from '@mui/material/FormControlLabel';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { Amount, MealSlot, PlannerMeal, Product, RecipeSummary, Unit } from '../../api/client';
import { ApiError } from '../../api/client';
import {
  useCreateMealPlanEntry,
  useHouseholdSlotAttendance,
  useMembers,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { displayUnit } from '../../components/UnitSelect';
import { parseIsoDate } from './date';
import { FoodSearch, type FoodChoice } from './FoodSearch';
import { MAIN_SLOTS, SLOTS } from './slots';

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
  const create = useCreateMealPlanEntry();
  const update = useUpdateMealPlanEntry();
  const [plannedOn, setPlannedOn] = useState(meal?.planned_on ?? date);
  const [plannedSlot, setPlannedSlot] = useState<MealSlot>(meal?.slot ?? slot);
  const [plannedTime, setPlannedTime] = useState(meal?.planned_time ?? '');
  const mealNoun = plannedSlot === 'snacks' ? 'snack' : 'meal';
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
  const [showWhen, setShowWhen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = create.isPending || update.isPending;

  const attendance = useHouseholdSlotAttendance(
    household ? plannedOn : '',
    household ? plannedSlot : '',
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
      amount: { kind: 'measure', unit: next.nutrition.basis?.unit ?? next.package_quantity?.unit ?? 'g', value: next.nutrition.basis?.amount ?? 100 },
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

  async function save() {
    if (foods.length === 0) {
      setError('Add at least one food.');
      return;
    }
    if (foods.some((food) => !Number.isFinite(food.amount.value) || food.amount.value <= 0)) {
      setError('Every food needs an amount greater than zero.');
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
        setError('Custom portions must add up to the total amount for each food.');
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
            planned_on: plannedOn,
            slot: plannedSlot,
            planned_time: plannedTime || null,
            portioning: household ? (customPortions ? 'custom' : 'equal') : undefined,
            components,
            ...(household ? { participants, guest_count: guestCount, guest_allocations: guestAllocations } : {}),
          },
        });
      } else {
        await create.mutateAsync({
          planned_on: plannedOn,
          slot: plannedSlot,
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

  const slotOptions = household ? MAIN_SLOTS : SLOTS;
  const whenSummary = [
    parseIsoDate(plannedOn).toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' }),
    SLOTS.find((option) => option.value === plannedSlot)?.label ?? plannedSlot,
    plannedTime || 'add time',
  ].join(' · ');

  return (
    <FormDialog open={open} onClose={busy ? undefined : onClose} fullWidth maxWidth="md">
      <DialogTitle>{meal ? `Edit ${mealNoun}` : household ? 'Plan household meal' : `Plan ${mealNoun}`}</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={3}>
          {error ? <Alert severity="error">{error}</Alert> : null}
          <Box>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
              <Typography color={plannedTime ? 'text.primary' : 'text.secondary'}>{whenSummary}</Typography>
              <Button size="small" onClick={() => setShowWhen((current) => !current)}>{showWhen ? 'Done' : 'Edit'}</Button>
            </Stack>
            {showWhen ? (
              <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ mt: 2 }}>
                <TextField label="Date" type="date" value={plannedOn} onChange={(event) => setPlannedOn(event.target.value)} slotProps={{ inputLabel: { shrink: true } }} fullWidth />
                <TextField select label="Meal" value={plannedSlot} onChange={(event) => setPlannedSlot(event.target.value as MealSlot)} fullWidth>
                  {slotOptions.map((option) => <MenuItem key={option.value} value={option.value}>{option.label}</MenuItem>)}
                </TextField>
                <TextField label="Time (optional)" type="time" value={plannedTime} onChange={(event) => setPlannedTime(event.target.value)} slotProps={{ inputLabel: { shrink: true } }} fullWidth />
              </Stack>
            ) : null}
          </Box>

          {household ? (
            <Box>
              <Stack direction="row" spacing={1} sx={{ alignItems: 'baseline', mb: 1 }}>
                <Typography variant="h3">Who is eating?</Typography>
                {attendance.isFetching ? (
                  <Typography variant="caption" color="text.secondary">Checking who's free…</Typography>
                ) : null}
              </Stack>
              <Stack sx={{ gap: 0.25 }}>
                {visibleMembers.map((member) => {
                  const blocked = memberBlockedReason(member.id);
                  return (
                    <FormControlLabel
                      key={member.id}
                      sx={{ alignItems: 'flex-start', mr: 0 }}
                      control={
                        <Checkbox
                          sx={{ pt: 0.5 }}
                          checked={selectedMembers.includes(member.id)}
                          disabled={Boolean(blocked) || attendance.isFetching}
                          onChange={(_, checked) => setSelectedMembers((current) => checked ? [...current, member.id] : current.filter((id) => id !== member.id))}
                        />
                      }
                      label={
                        <Box sx={{ py: 0.5 }}>
                          <Typography color={blocked ? 'text.disabled' : 'text.primary'}>{member.display_name}</Typography>
                          {blocked ? <Typography variant="caption" color="text.secondary">{blocked}</Typography> : null}
                        </Box>
                      }
                    />
                  );
                })}
              </Stack>
              <TextField
                label="Guests"
                type="number"
                value={guestCount}
                onChange={(event) => setGuestCount(Math.max(0, Number(event.target.value) || 0))}
                slotProps={{ htmlInput: { min: 0, step: 1 } }}
                sx={{ mt: 1, width: 160 }}
              />
            </Box>
          ) : null}

          <Box>
            <Typography variant="h3" sx={{ mb: 1 }}>Food for the {mealNoun}</Typography>
            <Stack spacing={2}>
              <FoodSearch
                onPick={addFood}
                excludeProductIds={foods.filter((food) => food.itemKind === 'product').map((food) => food.itemId)}
                excludeRecipeIds={foods.filter((food) => food.itemKind === 'recipe').map((food) => food.itemId)}
              />
              {foods.length === 0 ? (
                <Typography variant="body2" color="text.secondary">No food added yet.</Typography>
              ) : (
                <Stack spacing={1.5}>
                  {foods.map((food) => (
                    <Stack key={food.componentId} direction={{ xs: 'column', sm: 'row' }} spacing={1.5} sx={{ alignItems: { sm: 'center' } }}>
                      <Typography sx={{ flex: 1, minWidth: 180 }}>{food.name}</Typography>
                      <TextField
                        label="Amount"
                        type="number"
                        value={amountValue(food.amount)}
                        onChange={(event) => setFoodAmount(food.componentId, withAmountValue(food.amount, event.target.value))}
                        slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                        sx={{ width: 140 }}
                      />
                      {food.amount.kind === 'measure' ? (
                        <TextField select label="Unit" value={food.amount.unit} onChange={(event) => setFoodAmount(food.componentId, { kind: 'measure', value: food.amount.value, unit: event.target.value as Unit })} sx={{ width: 130 }}>
                          {UNITS.map((unit) => <MenuItem key={unit} value={unit}>{displayUnit(unit)}</MenuItem>)}
                        </TextField>
                      ) : (
                        <Typography color="text.secondary" sx={{ width: 130 }}>
                          {food.amount.value === 1 ? 'serving' : 'servings'}
                        </Typography>
                      )}
                      <IconButton aria-label={`Remove ${food.name}`} onClick={() => setFoods((current) => current.filter((candidate) => candidate.componentId !== food.componentId))}><DeleteIcon /></IconButton>
                    </Stack>
                  ))}
                </Stack>
              )}
            </Stack>
          </Box>

          {household && diners > 0 && foods.length > 0 ? (
            <Box>
              <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                <Box>
                  <Typography variant="h3">Portions</Typography>
                  <Typography variant="body2" color="text.secondary">{customPortions ? 'Custom portions' : 'Split equally'}</Typography>
                </Box>
                {!customPortions ? <Button startIcon={<AddIcon />} onClick={enableCustomPortions}>Adjust portions</Button> : null}
              </Stack>
              {customPortions ? (
                <Stack spacing={2} sx={{ mt: 2 }}>
                  {foods.map((food) => (
                    <Box key={food.componentId}>
                      <Typography sx={{ mb: 1, fontWeight: 600 }}>{food.name}</Typography>
                      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5} sx={{ flexWrap: 'wrap' }}>
                        {selectedMembers.map((memberId) => (
                          <TextField
                            key={memberId}
                            label={visibleMembers.find((member) => member.id === memberId)?.display_name ?? 'Member'}
                            type="number"
                            value={portions[allocationKey(food.componentId, memberId)] ?? ''}
                            onChange={(event) => setPortions((current) => ({ ...current, [allocationKey(food.componentId, memberId)]: event.target.value }))}
                            sx={{ width: 150 }}
                          />
                        ))}
                        {guestCount > 0 ? (
                          <TextField label="Each guest" type="number" value={portions[allocationKey(food.componentId, 'guest')] ?? ''} onChange={(event) => setPortions((current) => ({ ...current, [allocationKey(food.componentId, 'guest')]: event.target.value }))} sx={{ width: 150 }} />
                        ) : null}
                      </Stack>
                    </Box>
                  ))}
                </Stack>
              ) : null}
            </Box>
          ) : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={busy}>Cancel</Button>
        <Button variant="contained" onClick={() => void save()} disabled={busy}>{meal ? 'Save changes' : `Plan ${mealNoun}`}</Button>
      </DialogActions>
    </FormDialog>
  );
}
