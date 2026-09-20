import AddIcon from '@mui/icons-material/AddOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Drawer from '@mui/material/Drawer';
import IconButton from '@mui/material/IconButton';
import InputBase from '@mui/material/InputBase';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { Amount, MealPlanComponent } from '../../../api/client';
import { ApiError } from '../../../api/client';
import { useCreateMealTemplateFromEntry, useDeleteGroup, useUpdateGroup } from '../../../api/queries';
import { FormDialog } from '../../../components/FormDialog';
import { IconTile } from '../../../components/IconTile';
import { formatAmount } from '../format';
import { PickerRowButton, PickerSearchField } from './pickerParts';
import { firstName, groupDisplayName, occasionTitle, toExistingGroupComponent } from './plannerWeek';
import type { GroupView, NewGroupComponent, OccasionView } from './types';
import { usePickerRows } from './usePickerRows';

function stepAmount(amount: Amount, direction: 1 | -1): Amount {
  if (amount.kind === 'measure') {
    const step = amount.unit === 'g' || amount.unit === 'ml' ? 10 : 1;
    return { ...amount, value: Math.max(step, amount.value + direction * step) };
  }
  return { ...amount, value: Math.max(1, amount.value + direction) };
}

function componentKey(component: MealPlanComponent): string {
  const withKind = component as MealPlanComponent & Record<string, string>;
  const idField = ['product_id', 'recipe_id', 'dish_recipe_id', 'ingredient_id', 'prepared_meal_id'].find(
    (field) => typeof withKind[field] === 'string',
  );
  return idField ? `${component.item_kind}:${withKind[idField]}` : `${component.item_kind}:${component.id}`;
}

function sharedWith(occasion: OccasionView, group: GroupView, component: MealPlanComponent): string | null {
  const key = componentKey(component);
  const item = occasion.cooking.find((candidate) => {
    const withKind = candidate as typeof candidate & Record<string, string>;
    const idField = ['product_id', 'recipe_id', 'dish_recipe_id', 'ingredient_id', 'prepared_meal_id'].find(
      (field) => typeof withKind[field] === 'string',
    );
    const candidateKey = idField ? `${candidate.item_kind}:${withKind[idField]}` : null;
    return candidateKey === key;
  });
  if (!item || item.group_ids.length <= 1) return null;
  const otherGroupId = item.group_ids.find((id) => id !== group.id);
  const otherGroup = occasion.groups.find((candidate) => candidate.id === otherGroupId);
  const otherDiner = otherGroup?.participants[0];
  if (!otherDiner) return null;
  return `Also in ${firstName(otherDiner.name)}'s meal`;
}

export function MealSheet({
  open,
  group: groupProp,
  occasion: occasionProp,
  sheet,
  onClose,
}: {
  open: boolean;
  group: GroupView | null;
  occasion: OccasionView | null;
  sheet: boolean;
  onClose: () => void;
}) {
  const updateGroup = useUpdateGroup();
  const deleteGroup = useDeleteGroup();
  const saveTemplate = useCreateMealTemplateFromEntry();
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [editingName, setEditingName] = useState(false);
  const [nameText, setNameText] = useState('');
  const [saving, setSaving] = useState(false);
  const [templateName, setTemplateName] = useState('');

  const { rows } = usePickerRows(query, occasionProp?.planned_on ?? '');
  const busy = updateGroup.isPending || deleteGroup.isPending || saveTemplate.isPending;

  if (!open || !groupProp || !occasionProp) return null;
  const group = groupProp;
  const occasion = occasionProp;

  const derived = groupDisplayName(group);
  const title = derived.tail ? `${derived.head} ${derived.tail}` : derived.head;
  const dinerNames = group.participants.map((participant) => firstName(participant.name)).join(' and ');
  const subtitle = [dinerNames || null, occasionTitle(occasion)].filter(Boolean).join(' · ');

  async function run(action: () => Promise<unknown>, fallback: string) {
    try {
      setError(null);
      await action();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : fallback);
    }
  }

  function commit(components: NewGroupComponent[]) {
    void run(
      () => updateGroup.mutateAsync({ id: group.id, body: { components, revision: group.revision } }),
      'Could not update this meal.',
    );
  }

  function step(component: MealPlanComponent, direction: 1 | -1) {
    const amount = stepAmount(component.amount, direction);
    const cookingServings =
      component.item_kind === 'recipe' && amount.kind === 'servings'
        ? amount.value === group.serves
          ? null
          : amount.value
        : null;
    const next = group.components.map((candidate) => {
      const base = toExistingGroupComponent(candidate);
      if (!base) return null;
      if (candidate.id !== component.id) return base;
      return { ...base, amount, cooking_servings: cookingServings };
    });
    commit(next.filter((value): value is NewGroupComponent => value !== null));
  }

  function remove(component: MealPlanComponent) {
    const next = group.components
      .filter((candidate) => candidate.id !== component.id)
      .map(toExistingGroupComponent)
      .filter((value): value is NewGroupComponent => value !== null);
    commit(next);
  }

  function addRow(added: NewGroupComponent) {
    const existing = group.components
      .map(toExistingGroupComponent)
      .filter((value): value is NewGroupComponent => value !== null);
    setQuery('');
    commit([...existing, added]);
  }

  function commitName() {
    setEditingName(false);
    const value = nameText.trim() || null;
    if (value === group.label) return;
    void run(
      () => updateGroup.mutateAsync({ id: group.id, body: { label: value, revision: group.revision } }),
      'Could not rename this meal.',
    );
  }

  function removeMeal() {
    void run(async () => {
      await deleteGroup.mutateAsync({ id: group.id, revision: group.revision });
      onClose();
    }, 'Could not delete this meal.');
  }

  function saveAsMeal() {
    void run(async () => {
      await saveTemplate.mutateAsync({ entryId: group.id, name: templateName.trim() || title });
      setSaving(false);
    }, 'Could not save this as a meal.');
  }

  const header = (
    <Stack direction="row" sx={{ alignItems: 'flex-start', justifyContent: 'space-between', gap: 2 }}>
      <Box>
        <Typography variant="h2" component="h2">
          {title}
        </Typography>
        {subtitle ? (
          <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
            {subtitle}
          </Typography>
        ) : null}
      </Box>
      <IconButton aria-label="Close" onClick={onClose} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: '10px' }}>
        <CloseIcon fontSize="small" />
      </IconButton>
    </Stack>
  );

  const body = (
    <Stack spacing={1.5}>
      {error ? (
        <Typography variant="body2" color="error.main">
          {error}
        </Typography>
      ) : null}
      <Stack>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', fontWeight: 600, pb: 1 }}
        >
          Food
        </Typography>
        {group.components.map((component) => {
          const amount = formatAmount(component.amount);
          const caption = sharedWith(occasion, group, component);
          return (
            <Box
              key={component.id}
              sx={{
                display: 'grid',
                gridTemplateColumns: '26px 1fr auto auto',
                gap: 1.5,
                alignItems: 'center',
                py: 1.25,
                borderTop: '1px solid',
                borderColor: 'divider',
              }}
            >
              <IconTile
                concept={component.item_kind === 'recipe' ? 'recipe' : component.item_kind === 'dish' ? 'dish' : 'food'}
                tone={component.item_kind === 'recipe' ? 'primary' : component.item_kind === 'dish' ? 'secondary' : 'neutral'}
                size={26}
              />
              <Box sx={{ minWidth: 0 }}>
                <Typography sx={{ fontWeight: 500 }} noWrap>
                  {component.item_name}
                </Typography>
                {caption ? (
                  <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                    {caption}
                  </Typography>
                ) : null}
              </Box>
              <Stack direction="row" spacing={0.25} sx={{ alignItems: 'center', border: '1px solid', borderColor: 'divider', borderRadius: '10px', p: 0.25 }}>
                <IconButton
                  size="small"
                  aria-label={`Less ${component.item_name}`}
                  disabled={busy}
                  onClick={() => step(component, -1)}
                >
                  <RemoveIcon sx={{ fontSize: 16 }} />
                </IconButton>
                <Typography variant="body2" className="numeral" sx={{ minWidth: 58, textAlign: 'center' }}>
                  {amount}
                </Typography>
                <IconButton
                  size="small"
                  aria-label={`More ${component.item_name}`}
                  disabled={busy}
                  onClick={() => step(component, 1)}
                >
                  <AddIcon sx={{ fontSize: 16 }} />
                </IconButton>
              </Stack>
              <IconButton
                size="small"
                aria-label={`Take ${component.item_name} out of this meal`}
                disabled={busy}
                onClick={() => remove(component)}
                sx={{ color: 'text.disabled' }}
              >
                <CloseIcon sx={{ fontSize: 16 }} />
              </IconButton>
            </Box>
          );
        })}
        <Box sx={{ mt: 1.25 }}>
          <PickerSearchField
            value={query}
            onChange={setQuery}
            onKeyDown={() => {}}
            placeholder="Add food to this meal"
          />
          {query.trim() ? (
            <Stack sx={{ border: '1px solid', borderColor: 'divider', borderRadius: '10px', mt: 1, overflow: 'hidden' }}>
              {rows
                .filter((row) => row.section === 'matches')
                .map((row) => {
                  const added = row.group.components?.[0];
                  if (!added) return null;
                  return <PickerRowButton key={row.id} row={row} selected={false} onPick={() => addRow(added)} />;
                })}
            </Stack>
          ) : null}
        </Box>
      </Stack>
      <Box>
        <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2, py: 1.5, borderTop: '1px solid', borderColor: 'divider' }}>
          <Typography variant="body2" color="text.secondary">
            Name
          </Typography>
          {editingName ? (
            <InputBase
              autoFocus
              value={nameText}
              placeholder={title}
              inputProps={{ 'aria-label': 'Meal name' }}
              onChange={(event) => setNameText(event.target.value)}
              onBlur={commitName}
              onKeyDown={(event) => {
                if (event.key === 'Enter') commitName();
                if (event.key === 'Escape') setEditingName(false);
              }}
              sx={{ fontSize: '0.875rem', flexGrow: 1, textAlign: 'right', border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1 }}
            />
          ) : (
            <Box
              component="button"
              type="button"
              onClick={() => {
                setNameText(group.label ?? '');
                setEditingName(true);
              }}
              sx={{ background: 'none', border: 0, p: 0, font: 'inherit', cursor: 'pointer', textAlign: 'right' }}
            >
              <Typography variant="body2" sx={{ color: group.label ? 'text.primary' : 'text.disabled' }}>
                {group.label ?? title}
              </Typography>
            </Box>
          )}
        </Stack>
      </Box>
      {saving ? (
        <Stack direction="row" spacing={1}>
          <InputBase
            autoFocus
            value={templateName}
            placeholder={title}
            inputProps={{ 'aria-label': 'Saved meal name' }}
            onChange={(event) => setTemplateName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') saveAsMeal();
              if (event.key === 'Escape') setSaving(false);
            }}
            sx={{ flexGrow: 1, border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1 }}
          />
          <Button onClick={saveAsMeal}>Save</Button>
        </Stack>
      ) : null}
    </Stack>
  );

  const footer = (
    <Stack direction="row" sx={{ justifyContent: 'space-between', width: '100%', borderTop: '1px solid', borderColor: 'divider', pt: 1.5 }}>
      <Button color="error" onClick={removeMeal} sx={{ ml: -1.25 }}>
        Delete meal
      </Button>
      <Stack direction="row" spacing={1}>
        <Button
          onClick={() => {
            setTemplateName(title);
            setSaving(true);
          }}
        >
          Save as a meal
        </Button>
        <Button variant="contained" onClick={onClose}>
          Done
        </Button>
      </Stack>
    </Stack>
  );

  if (sheet) {
    return (
      <Drawer anchor="bottom" open onClose={onClose} slotProps={{ paper: { sx: { borderRadius: '14px 14px 0 0', maxHeight: '92vh' } } }}>
        <Stack spacing={3} sx={{ px: 2.5, pt: 3, pb: 2 }}>
          {header}
          {body}
          {footer}
        </Stack>
      </Drawer>
    );
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth={false} slotProps={{ paper: { sx: { width: 480, maxWidth: 'calc(100vw - 32px)' } } }}>
      <DialogTitle component="div" sx={{ px: 4, pt: 3.5, pb: 1 }}>
        {header}
      </DialogTitle>
      <DialogContent sx={{ px: 4, pt: 2 }}>{body}</DialogContent>
      <DialogActions sx={{ px: 4, pb: 3, pt: 0 }}>{footer}</DialogActions>
    </FormDialog>
  );
}
