import WarningIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import type { PlannerMeal } from '../../api/client';
import { FormDialog } from '../../components/FormDialog';
import { IconTile } from '../../components/IconTile';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { KindChip, type Kind } from '../../components/KindChip';
import { fullDayLabel } from './date';
import { formatAmount } from './format';
import { mealTitle } from './MealRow';
import { labelForSlot } from './slots';

type Allocation = PlannerMeal['people'][number]['allocations'][number]['allocated'];

function formatAllocation(amount: Allocation): string {
  const value = Number(amount.value);
  if (amount.kind === 'servings') return value === 1 ? '1 serving' : `${value} servings`;
  if (amount.kind === 'packs') return value === 1 ? '1 pack' : `${value} packs`;
  return amount.unit ? `${value} ${amount.unit}` : String(value);
}

function allocationsLabel(allocations: PlannerMeal['people'][number]['allocations']): string {
  if (allocations.length === 0) return 'Serving planned';
  return allocations.map((allocation) => formatAllocation(allocation.allocated)).join(' · ');
}

function foodPresentation(food: PlannerMeal['foods'][number]): { concept: 'recipe' | 'dish' | 'food'; kind: Kind } {
  if (food.item_kind === 'recipe') return { concept: 'recipe', kind: 'recipe' };
  if (food.item_kind === 'dish') return { concept: 'dish', kind: 'dish' };
  if (food.item_kind === 'product') return { concept: 'food', kind: 'product' };
  return { concept: 'food', kind: 'food' };
}

function DetailRow({ leading, title, chip, trailing }: {
  leading: ReactNode;
  title: string;
  chip?: ReactNode;
  trailing: string;
}) {
  return (
    <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', px: 2, py: 1.5 }}>
      {leading}
      <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexGrow: 1, minWidth: 0, flexWrap: 'wrap' }}>
        <Typography variant="body2" sx={{ fontWeight: 500 }}>{title}</Typography>
        {chip}
      </Stack>
      <Typography variant="caption" color="text.secondary" className="numeral" sx={{ flexShrink: 0 }}>
        {trailing}
      </Typography>
    </Stack>
  );
}

export function shortageWarning(meal: PlannerMeal): string | null {
  const shortages = meal.foods.filter((food) => food.shortage);
  return shortages.length > 0
    ? `Not enough servings for ${shortages.map((food) => food.item_name).join(', ')}`
    : null;
}

export function MealSheet({
  meal,
  onClose,
  onEdit,
  onDelete,
  onLeave,
  onJoin,
  busy = false,
}: {
  meal: PlannerMeal;
  onClose: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onLeave: () => void;
  onJoin: () => void;
  busy?: boolean;
}) {
  const warning = shortageWarning(meal);
  const primary = meal.capabilities.can_edit
    ? 'edit'
    : meal.can_join
      ? 'join'
      : meal.can_opt_out
        ? 'leave'
        : null;
  const subtitle = [
    labelForSlot(meal.slot),
    fullDayLabel(meal.planned_on),
    meal.planned_time,
  ].filter(Boolean).join(' · ');

  function actionButton(action: 'edit' | 'join' | 'leave', label: string, onClick: () => void) {
    return (
      <Button key={action} variant={primary === action ? 'contained' : 'text'} onClick={onClick} disabled={busy}>
        {label}
      </Button>
    );
  }

  return (
    <FormDialog open onClose={busy ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">{mealTitle(meal)}</Typography>
        <Typography component="span" variant="body2" color="text.secondary" className="numeral" sx={{ display: 'block', mt: 0.5 }}>{subtitle}</Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          <Box>
            <Typography variant="overline" color="text.secondary">Eating</Typography>
            <Paper variant="outlined" sx={{ mt: 0.75, overflow: 'hidden' }}>
              <Stack divider={<Divider flexItem />}>
                {meal.people.map((person) => (
                  <DetailRow
                    key={person.member_id}
                    leading={<InitialsAvatar name={person.display_name} size={34} />}
                    title={person.display_name}
                    trailing={allocationsLabel(person.allocations)}
                  />
                ))}
                {meal.guest_groups.map((group) => (
                  <DetailRow
                    key={group.id}
                    leading={<InitialsAvatar name="Guests" size={34} />}
                    title={`${group.count} ${group.count === 1 ? 'guest' : 'guests'}`}
                    trailing={allocationsLabel(group.allocations)}
                  />
                ))}
              </Stack>
            </Paper>
          </Box>
          <Box>
            <Typography variant="overline" color="text.secondary">Food</Typography>
            <Paper variant="outlined" sx={{ mt: 0.75, overflow: 'hidden' }}>
              <Stack divider={<Divider flexItem />}>
                {meal.foods.map((food) => {
                  const presentation = foodPresentation(food);
                  return (
                    <DetailRow
                      key={food.id}
                      leading={<IconTile concept={presentation.concept} size={34} />}
                      title={food.item_name}
                      chip={<KindChip kind={presentation.kind} />}
                      trailing={formatAmount(food.amount)}
                    />
                  );
                })}
              </Stack>
            </Paper>
            {warning ? (
              <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mt: 1.25, color: 'warning.main' }}>
                <WarningIcon sx={{ fontSize: 17 }} />
                <Typography variant="caption">{warning}</Typography>
              </Stack>
            ) : null}
          </Box>
        </Stack>
      </DialogContent>
      <DialogActions sx={{ justifyContent: 'space-between', flexWrap: 'wrap', gap: 0.5 }}>
        <Box>
          {primary === 'edit' ? actionButton('edit', 'Edit meal', onEdit) : null}
          {primary === 'join' ? actionButton('join', 'Join this meal', onJoin) : null}
          {primary === 'leave' ? actionButton('leave', 'Leave this meal', onLeave) : null}
        </Box>
        <Stack direction="row" sx={{ flexWrap: 'wrap', justifyContent: 'flex-end' }}>
          {meal.capabilities.can_edit && primary !== 'edit' ? actionButton('edit', 'Edit meal', onEdit) : null}
          {meal.can_join && primary !== 'join' ? actionButton('join', 'Join this meal', onJoin) : null}
          {meal.can_opt_out && primary !== 'leave' ? actionButton('leave', 'Leave this meal', onLeave) : null}
          {meal.capabilities.can_delete ? <Button onClick={onDelete} disabled={busy}>Delete meal</Button> : null}
        </Stack>
      </DialogActions>
    </FormDialog>
  );
}
