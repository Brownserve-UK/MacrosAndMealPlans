import { useMemo, useState } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Checkbox from '@mui/material/Checkbox';
import Chip from '@mui/material/Chip';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import FormControlLabel from '@mui/material/FormControlLabel';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import WarningAmberIcon from '@mui/icons-material/WarningAmberOutlined';

import { InitialsAvatar } from '../../components/InitialsAvatar';
import { FormDialog } from '../../components/FormDialog';
import { useMembers, useSetMealPlanParticipants } from '../../api/queries';
import type { MealPlanEntry } from '../../api/client';
import type { components } from '../../api/schema';

type ComponentAmount =
  components['schemas']['MealParticipantAllocationRequest']['amount'];

type AnyAmount = { value: string | number; unit?: string | null; kind?: string };

function componentUnit(amount: AnyAmount): string | null {
  return 'unit' in amount && amount.unit ? amount.unit : null;
}

function amountLabel(amount: AnyAmount | undefined | null): string {
  if (!amount) return '—';
  const value = Number(amount.value);
  const unit = componentUnit(amount);
  return unit ? `${value} ${unit}` : `${value} servings`;
}

export function ParticipantSummary({ entry }: { entry: MealPlanEntry }) {
  const [open, setOpen] = useState(false);
  const members = useMembers({ include_archived: false });
  const save = useSetMealPlanParticipants();

  const shortage = entry.components.some((component) => component.preparation.shortage);
  const leftoverComponents = entry.components.filter(
    (component) => component.preparation.leftover && Number(component.preparation.leftover.value) > 0,
  );

  const initialSelection = useMemo(() => {
    const map = new Map<string, Map<string, string>>();
    for (const participant of entry.participants) {
      const allocations = new Map<string, string>();
      for (const allocation of participant.allocations) {
        allocations.set(allocation.component_id, String(Number(allocation.allocated.value)));
      }
      map.set(participant.member_id, allocations);
    }
    return map;
  }, [entry.participants]);

  const [selection, setSelection] = useState(initialSelection);

  function openDialog() {
    setSelection(new Map([...initialSelection].map(([id, allocations]) => [id, new Map(allocations)])));
    setOpen(true);
  }

  function toggleMember(memberId: string, checked: boolean) {
    setSelection((current) => {
      const next = new Map([...current].map(([id, allocations]) => [id, new Map(allocations)]));
      if (checked) {
        const allocations = new Map<string, string>();
        for (const component of entry.components) {
          allocations.set(
            component.id,
            componentUnit(component.amount) ? String(Number(component.amount.value)) : '1',
          );
        }
        next.set(memberId, allocations);
      } else {
        next.delete(memberId);
      }
      return next;
    });
  }

  function setAllocation(memberId: string, componentId: string, value: string) {
    setSelection((current) => {
      const next = new Map([...current].map(([id, allocations]) => [id, new Map(allocations)]));
      next.get(memberId)?.set(componentId, value);
      return next;
    });
  }

  async function submit() {
    await save.mutateAsync({
      id: entry.id,
      revision: entry.revision,
      body: {
        participants: [...selection].map(([memberId, allocations]) => ({
          member_id: memberId,
          allocations: entry.components.map((component) => {
            const raw = Number(allocations.get(component.id) ?? '1');
            const unit = componentUnit(component.amount);
            return {
              component_id: component.id,
              amount: unit
                ? ({ kind: 'measure', value: raw, unit } as ComponentAmount)
                : ({ kind: 'servings', value: raw } as ComponentAmount),
            };
          }),
        })),
      },
    });
    setOpen(false);
  }

  return (
    <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
      <Stack direction="row" spacing={-0.5} sx={{ alignItems: 'center' }}>
        {entry.participants.slice(0, 4).map((participant) => (
          <Box key={participant.member_id} sx={{ ml: -0.5 }} title={participant.display_name}>
            <InitialsAvatar name={participant.display_name || '?'} size={24} />
          </Box>
        ))}
      </Stack>
      {shortage ? (
        <Chip
          size="small"
          color="warning"
          icon={<WarningAmberIcon />}
          label="Not enough servings"
        />
      ) : leftoverComponents.length > 0 ? (
        <Chip
          size="small"
          variant="outlined"
          label={`${amountLabel(leftoverComponents[0]!.preparation.leftover ?? undefined)} left over`}
        />
      ) : null}
      <Button size="small" onClick={openDialog}>
        Eating
      </Button>

      <FormDialog open={open} onClose={() => setOpen(false)} fullWidth maxWidth="sm">
        <DialogTitle>Who is eating this meal?</DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2}>
            {(members.data?.items ?? []).map((member) => {
              const allocations = selection.get(member.id);
              const eating = allocations != null;
              return (
                <Box key={member.id}>
                  <FormControlLabel
                    control={
                      <Checkbox
                        checked={eating}
                        onChange={(event) => toggleMember(member.id, event.target.checked)}
                      />
                    }
                    label={member.display_name}
                  />
                  {eating ? (
                    <Stack spacing={1} sx={{ pl: 4 }}>
                      {entry.components.map((component) => (
                        <Stack
                          key={component.id}
                          direction="row"
                          spacing={1}
                          sx={{ alignItems: 'center' }}
                        >
                          <Typography variant="body2" sx={{ flex: 1 }}>
                            {component.item_name}
                          </Typography>
                          <TextField
                            size="small"
                            type="number"
                            label={componentUnit(component.amount) ?? 'servings'}
                            value={allocations?.get(component.id) ?? '1'}
                            onChange={(event) =>
                              setAllocation(member.id, component.id, event.target.value)
                            }
                            sx={{ width: 120 }}
                            slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                          />
                        </Stack>
                      ))}
                    </Stack>
                  ) : null}
                </Box>
              );
            })}
            <Box>
              {entry.components.map((component) => (
                <Typography
                  key={component.id}
                  variant="caption"
                  color="text.secondary"
                  sx={{ display: 'block' }}
                >
                  {component.item_name}: prepared {amountLabel(component.preparation.prepared)}
                  {component.preparation.allocated
                    ? ` · allocated ${amountLabel(component.preparation.allocated)}`
                    : ''}
                  {component.preparation.leftover
                    ? ` · ${amountLabel(component.preparation.leftover)} left over`
                    : ''}
                </Typography>
              ))}
            </Box>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setOpen(false)}>Cancel</Button>
          <Button variant="contained" onClick={submit} disabled={save.isPending}>
            Save
          </Button>
        </DialogActions>
      </FormDialog>
    </Stack>
  );
}
