import AddIcon from '@mui/icons-material/AddOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import MoreIcon from '@mui/icons-material/MoreHorizOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import useMediaQuery from '@mui/material/useMediaQuery';
import { useTheme } from '@mui/material/styles';
import { useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import {
  ApiError,
  type Amount,
  type MealPlanEntry,
  type MealSlot,
  type Product,
} from '../../api/client';
import {
  useCreateMealPlanEntry,
  useDeleteMealPlanEntry,
  useMarkMealPlanEaten,
  useMarkMealPlanNotEaten,
  useProduct,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { MaybeNumber } from '../../components/Unknown';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from '../diary/AmountFields';
import { combineDateTime, nowTime, parseIsoDate } from '../diary/date';
import { formatAmount } from '../diary/format';
import { ProductPicker } from '../diary/ProductPicker';

export const SLOTS: { value: MealSlot; label: string }[] = [
  { value: 'breakfast', label: 'Breakfast' },
  { value: 'lunch', label: 'Lunch' },
  { value: 'dinner', label: 'Dinner' },
  { value: 'snacks', label: 'Snacks' },
];

type ComponentDraft = {
  key: string;
  componentId?: string;
  productId: string;
  product: Product | null;
  amount: AmountDraft;
};

type Draft = {
  date: string;
  time: string;
  slot: MealSlot;
  components: ComponentDraft[];
};

type Stage = 'edit' | 'eaten' | 'not_eaten' | 'delete';

function amountToDraft(amount: Amount): AmountDraft {
  return amount.kind === 'measure'
    ? { kind: 'measure', value: String(amount.value), unit: amount.unit }
    : { kind: amount.kind, value: String(amount.value), unit: 'g' };
}

function componentDraft(
  productId = '',
  amount: AmountDraft = amountDraftFrom(null),
  componentId?: string,
): ComponentDraft {
  return { key: crypto.randomUUID(), componentId, productId, product: null, amount };
}

function initialDraft(entry: MealPlanEntry | null, date: string, slot: MealSlot): Draft {
  if (!entry) return { date, time: '', slot, components: [componentDraft()] };
  return {
    date: entry.planned_on,
    time: entry.planned_time ?? '',
    slot: entry.slot,
    components: entry.components.map((component) =>
      componentDraft(component.product_id, amountToDraft(component.amount), component.id),
    ),
  };
}

function labelForSlot(slot: MealSlot) {
  return SLOTS.find((candidate) => candidate.value === slot)?.label ?? slot;
}

function formattedDate(date: string) {
  return parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function validationErrors(draft: Draft) {
  const errors: Record<string, string> = {};
  if (!draft.date) errors.planned_on = 'Pick a date';
  draft.components.forEach((component, index) => {
    if (!component.productId) errors[`components.${index}.product_id`] = 'Pick a product';
    const amountErrors = validateAmountDraft(component.amount);
    if (amountErrors.amount) errors[`components.${index}.amount`] = amountErrors.amount;
  });
  if (draft.components.length === 0) errors.components = 'Add at least one product';
  return errors;
}

function ComponentEditor({
  draft,
  index,
  errors,
  removable,
  productEditable,
  onChange,
  onRemove,
}: {
  draft: ComponentDraft;
  index: number;
  errors: Record<string, string>;
  removable: boolean;
  productEditable: boolean;
  onChange: (next: ComponentDraft) => void;
  onRemove: () => void;
}) {
  const existing = useProduct(draft.productId, {
    enabled: Boolean(draft.productId) && !draft.product,
  });
  const product = draft.product ?? existing.data ?? null;
  const prefix = `components.${index}`;

  return (
    <Box sx={{ py: 2.25 }}>
      <Stack spacing={2}>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'flex-start' }}>
          <Box sx={{ flexGrow: 1 }}>
            <ProductPicker
              value={product}
              onChange={(next) =>
                onChange({
                  ...draft,
                  productId: next?.id ?? '',
                  product: next,
                  amount: amountDraftFrom(next),
                })
              }
              error={Boolean(errors[`${prefix}.product_id`])}
              helperText={errors[`${prefix}.product_id`]}
              autoFocus={index === 0 && !draft.productId}
              disabled={!productEditable}
            />
          </Box>
          {removable ? (
            <IconButton aria-label="Remove product" onClick={onRemove} sx={{ mt: 0.25 }}>
              <DeleteIcon />
            </IconButton>
          ) : null}
        </Stack>
        {product ? (
          <AmountFields
            product={product}
            draft={draft.amount}
            errors={{ amount: errors[`${prefix}.amount`] ?? '' }}
            onChange={(amount) => onChange({ ...draft, amount })}
          />
        ) : null}
      </Stack>
    </Box>
  );
}

function ResolvedEntry({ entry }: { entry: MealPlanEntry }) {
  const eaten = entry.status === 'eaten';
  return (
    <Stack spacing={2.5}>
      <Box>
        <Typography sx={{ fontWeight: 650, color: eaten ? 'success.main' : 'text.secondary' }}>
          {eaten ? 'Eaten' : 'Not eaten'}
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.25 }}>
          {formattedDate(entry.planned_on)}
          {entry.planned_time ? ` at ${entry.planned_time.slice(0, 5)}` : ''}
        </Typography>
      </Box>
      <Divider />
      <Stack divider={<Divider />}>
        {entry.components.map((component) => (
          <Box
            key={component.id}
            sx={{
              display: 'grid',
              gridTemplateColumns: 'minmax(0, 1fr) auto',
              gap: 2,
              alignItems: 'start',
              py: 1.75,
            }}
          >
            <Box>
              <Typography sx={{ fontWeight: 600 }}>{component.product_name}</Typography>
              <Typography variant="body2" color="text.secondary">
                {formatAmount(component.consumption_record?.amount ?? component.amount)}
              </Typography>
              {component.consumption_record ? (
                <Link
                  to="/diary/$memberId/$date"
                  params={{
                    memberId: entry.member_id,
                    date: component.consumption_record.consumed_on,
                  }}
                  className="app-link"
                >
                  <Typography component="span" variant="caption">
                    View in diary
                  </Typography>
                </Link>
              ) : null}
            </Box>
            {eaten ? (
              <Typography className="numeral" variant="body2" sx={{ fontWeight: 600 }}>
                <MaybeNumber
                  value={
                    component.consumption_record?.nutrition.energy_kcal ??
                    component.nutrition.energy_kcal
                  }
                  fractionDigits={0}
                />{' '}
                kcal
              </Typography>
            ) : null}
          </Box>
        ))}
      </Stack>
      <Typography variant="caption" color="text.secondary">
        Resolved meals are retained as recorded and cannot be changed.
      </Typography>
    </Stack>
  );
}

function Confirmation({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <Box sx={{ py: 1 }}>
      <Typography variant="h3">{title}</Typography>
      <Typography color="text.secondary" sx={{ mt: 1, maxWidth: 520 }}>
        {description}
      </Typography>
    </Box>
  );
}

export function MealPlanEntryDialog({
  open,
  onClose,
  memberId,
  date,
  slot,
  entry,
}: {
  open: boolean;
  onClose: () => void;
  memberId: string;
  date: string;
  slot: MealSlot;
  entry: MealPlanEntry | null;
}) {
  const theme = useTheme();
  const queryClient = useQueryClient();
  const fullScreen = useMediaQuery(theme.breakpoints.down('sm'));
  const create = useCreateMealPlanEntry();
  const update = useUpdateMealPlanEntry();
  const remove = useDeleteMealPlanEntry();
  const markEaten = useMarkMealPlanEaten();
  const markNotEaten = useMarkMealPlanNotEaten();
  const [baseline] = useState(() => initialDraft(entry, date, slot));
  const [draft, setDraft] = useState(baseline);
  const [stage, setStage] = useState<Stage>('edit');
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [actualDate, setActualDate] = useState(entry?.planned_on ?? date);
  const [actualTime, setActualTime] = useState(entry?.planned_time ?? nowTime());
  const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);

  const busy =
    create.isPending ||
    update.isPending ||
    remove.isPending ||
    markEaten.isPending ||
    markNotEaten.isPending;
  const comparable = (value: Draft) => ({
    date: value.date,
    time: value.time,
    slot: value.slot,
    components: value.components.map((component) => ({
      componentId: component.componentId,
      productId: component.productId,
      amount: component.amount,
    })),
  });
  const dirty = JSON.stringify(comparable(draft)) !== JSON.stringify(comparable(baseline));

  function close() {
    if (!busy) onClose();
  }

  function report(caught: unknown) {
    if (caught instanceof ApiError) {
      if (caught.isConflict) setConflict(caught);
      else if (Object.keys(caught.fieldErrors).length > 0) setErrors(caught.fieldErrors);
      else setFailure(caught.message);
    } else setFailure('Could not save the meal.');
  }

  function componentsBody() {
    return draft.components.map((component) => ({
      product_id: component.productId,
      amount: draftToAmount(component.amount)!,
    }));
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validationErrors(draft);
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    try {
      if (entry) {
        await update.mutateAsync({
          id: entry.id,
          revision: entry.revision,
          body: {
            planned_on: draft.date,
            planned_time: draft.time || null,
            slot: draft.slot,
            components: componentsBody(),
          },
        });
      } else {
        await create.mutateAsync({
          member_id: memberId,
          planned_on: draft.date,
          planned_time: draft.time || null,
          slot: draft.slot,
          components: componentsBody(),
        });
      }
      onClose();
    } catch (caught) {
      report(caught);
    }
  }

  async function deleteEntry() {
    if (!entry) return;
    try {
      await remove.mutateAsync({ id: entry.id, revision: entry.revision, memberId });
      onClose();
    } catch (caught) {
      report(caught);
    }
  }

  async function confirmNotEaten() {
    if (!entry) return;
    try {
      await markNotEaten.mutateAsync({ id: entry.id, revision: entry.revision });
      onClose();
    } catch (caught) {
      report(caught);
    }
  }

  async function confirmEaten() {
    if (!entry) return;
    const found = validationErrors(draft);
    if (!actualDate) found.consumed_on = 'Pick the date eaten';
    if (!actualTime) found.consumed_at = 'Pick the time eaten';
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    try {
      await markEaten.mutateAsync({
        id: entry.id,
        revision: entry.revision,
        body: {
          consumed_on: actualDate,
          consumed_at: combineDateTime(actualDate, actualTime),
          components: draft.components.map((component) => ({
            component_id: component.componentId!,
            amount: draftToAmount(component.amount)!,
          })),
        },
      });
      onClose();
    } catch (caught) {
      report(caught);
    }
  }

  const title = !entry
    ? 'Plan a meal'
    : entry.status !== 'planned'
      ? `${labelForSlot(entry.slot)} details`
      : stage === 'eaten'
        ? 'Confirm what was eaten'
        : stage === 'not_eaten'
          ? 'Mark as not eaten'
          : stage === 'delete'
            ? 'Delete planned meal'
            : 'Edit planned meal';
  const context = `${formattedDate(stage === 'eaten' ? actualDate : draft.date)} · ${labelForSlot(draft.slot)}`;

  return (
    <FormDialog open={open} onClose={close} maxWidth="sm" fullWidth fullScreen={fullScreen}>
      <form
        onSubmit={(event) => {
          if (stage === 'edit') void save(event);
          else event.preventDefault();
        }}
      >
        <DialogTitle sx={{ pb: 1.5 }}>
          <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'flex-start' }}>
            <Box>
              <Typography component="h2" variant="h2">
                {title}
              </Typography>
              {stage !== 'delete' && stage !== 'not_eaten' ? (
                <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
                  {context}
                </Typography>
              ) : null}
            </Box>
            {entry?.status === 'planned' && stage === 'edit' ? (
              <IconButton aria-label="More meal actions" onClick={(event) => setMenuAnchor(event.currentTarget)}>
                <MoreIcon />
              </IconButton>
            ) : null}
          </Stack>
        </DialogTitle>
        <Menu anchorEl={menuAnchor} open={Boolean(menuAnchor)} onClose={() => setMenuAnchor(null)}>
          <MenuItem
            disabled={dirty}
            onClick={() => {
              setMenuAnchor(null);
              setStage('not_eaten');
            }}
          >
            Mark as not eaten
          </MenuItem>
          <Divider />
          <MenuItem
            sx={{ color: 'error.main' }}
            onClick={() => {
              setMenuAnchor(null);
              setStage('delete');
            }}
          >
            Delete planned meal
          </MenuItem>
        </Menu>

        <DialogContent sx={{ pt: 1 }}>
          {failure ? (
            <Alert severity="error" sx={{ mb: 2 }}>
              {failure}
            </Alert>
          ) : null}

          {entry && entry.status !== 'planned' ? (
            <ResolvedEntry entry={entry} />
          ) : stage === 'not_eaten' ? (
            <Confirmation
              title="Keep it in the history?"
              description="The meal will remain visible as not eaten and will no longer count towards projected nutrition."
            />
          ) : stage === 'delete' ? (
            <Confirmation
              title="Remove this meal entirely?"
              description="This removes it from the plan and cannot be undone."
            />
          ) : (
            <Stack spacing={3}>
              {stage === 'eaten' ? (
                <Typography color="text.secondary">
                  Adjust the time or amounts if the meal differed from the plan, then confirm it.
                </Typography>
              ) : null}

              <Box>
                <Typography variant="overline" color="text.secondary">
                  {stage === 'eaten' ? 'When it was eaten' : 'When'}
                </Typography>
                <Box
                  sx={{
                    display: 'grid',
                    gridTemplateColumns: {
                      xs: '1fr',
                      sm: stage === 'edit' ? '1fr 1fr 1fr' : '1fr 1fr',
                    },
                    gap: 2,
                    mt: 1,
                  }}
                >
                  <TextField
                    type="date"
                    label={stage === 'eaten' ? 'Date eaten' : 'Date'}
                    value={stage === 'eaten' ? actualDate : draft.date}
                    onChange={(event) =>
                      stage === 'eaten'
                        ? setActualDate(event.target.value)
                        : setDraft({ ...draft, date: event.target.value })
                    }
                    error={Boolean(stage === 'eaten' ? errors.consumed_on : errors.planned_on)}
                    helperText={stage === 'eaten' ? errors.consumed_on : errors.planned_on}
                    slotProps={{ inputLabel: { shrink: true } }}
                    fullWidth
                  />
                  <TextField
                    type="time"
                    label={stage === 'eaten' ? 'Time eaten' : 'Time'}
                    value={stage === 'eaten' ? actualTime : draft.time}
                    onChange={(event) =>
                      stage === 'eaten'
                        ? setActualTime(event.target.value)
                        : setDraft({ ...draft, time: event.target.value })
                    }
                    helperText={stage === 'eaten' ? errors.consumed_at : 'Optional'}
                    error={Boolean(errors.consumed_at)}
                    slotProps={{ inputLabel: { shrink: true } }}
                    fullWidth
                  />
                  {stage === 'edit' ? (
                    <TextField
                      select
                      label="Meal"
                      value={draft.slot}
                      onChange={(event) =>
                        setDraft({ ...draft, slot: event.target.value as MealSlot })
                      }
                      fullWidth
                    >
                      {SLOTS.map((candidate) => (
                        <MenuItem key={candidate.value} value={candidate.value}>
                          {candidate.label}
                        </MenuItem>
                      ))}
                    </TextField>
                  ) : null}
                </Box>
              </Box>

              <Box>
                <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
                  <Typography variant="overline" color="text.secondary">
                    {stage === 'eaten' ? 'What was eaten' : 'Products'}
                  </Typography>
                  {stage === 'edit' ? (
                    <Button
                      size="small"
                      startIcon={<AddIcon />}
                      onClick={() =>
                        setDraft({
                          ...draft,
                          components: [...draft.components, componentDraft()],
                        })
                      }
                    >
                      Add product
                    </Button>
                  ) : null}
                </Stack>
                {errors.components ? (
                  <Typography variant="caption" color="error" sx={{ display: 'block', mt: 1 }}>
                    {errors.components}
                  </Typography>
                ) : null}
                <Stack divider={<Divider />}>
                  {draft.components.map((component, index) => (
                    <ComponentEditor
                      key={component.key}
                      draft={component}
                      index={index}
                      errors={errors}
                      removable={draft.components.length > 1 && stage === 'edit'}
                      productEditable={stage === 'edit'}
                      onChange={(next) => {
                        const components = [...draft.components];
                        components[index] = next;
                        setDraft({ ...draft, components });
                      }}
                      onRemove={() =>
                        setDraft({
                          ...draft,
                          components: draft.components.filter(
                            (_, candidate) => candidate !== index,
                          ),
                        })
                      }
                    />
                  ))}
                </Stack>
              </Box>
            </Stack>
          )}
        </DialogContent>

        <DialogActions sx={{ px: 3, py: 2.5 }}>
          {entry?.status !== 'planned' ? (
            <Button onClick={close} disabled={busy}>
              Close
            </Button>
          ) : stage === 'edit' ? (
            <>
              <Button onClick={close} disabled={busy} sx={{ mr: 'auto' }}>
                Cancel
              </Button>
              {entry && !dirty ? (
                <Button onClick={() => setStage('eaten')} variant="contained" disabled={busy}>
                  Mark eaten
                </Button>
              ) : (
                <Button type="submit" variant="contained" disabled={busy}>
                  {busy ? 'Saving…' : entry ? 'Save changes' : 'Add to plan'}
                </Button>
              )}
            </>
          ) : (
            <>
              <Button onClick={() => setStage('edit')} disabled={busy} sx={{ mr: 'auto' }}>
                Back
              </Button>
              {stage === 'eaten' ? (
                <Button onClick={confirmEaten} variant="contained" disabled={busy}>
                  {busy ? 'Confirming…' : 'Confirm eaten'}
                </Button>
              ) : stage === 'not_eaten' ? (
                <Button onClick={confirmNotEaten} color="warning" variant="contained" disabled={busy}>
                  {busy ? 'Confirming…' : 'Mark not eaten'}
                </Button>
              ) : (
                <Button onClick={deleteEntry} color="error" variant="contained" disabled={busy}>
                  {busy ? 'Deleting…' : 'Delete meal'}
                </Button>
              )}
            </>
          )}
        </DialogActions>
      </form>
      <ConflictDialog
        error={conflict}
        onDismiss={() => setConflict(null)}
        onReload={() => {
          setConflict(null);
          void queryClient.invalidateQueries({ queryKey: ['mealPlanWeek', memberId] });
          onClose();
        }}
      />
    </FormDialog>
  );
}
