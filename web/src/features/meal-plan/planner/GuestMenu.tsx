import CheckIcon from '@mui/icons-material/CheckOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import Drawer from '@mui/material/Drawer';
import Popover from '@mui/material/Popover';
import Stack from '@mui/material/Stack';
import type { KeyboardEvent, ReactNode } from 'react';
import { useState } from 'react';
import { PickerPromptStep, PickerRowButton, PickerSearchField, SectionHeading, type PickerRowContent } from './pickerParts';
import { conceptFor, dishLabel } from './plannerWeek';
import type { GroupView, NewGroup, PlannerGuest } from './types';
import { leftoversRow, useFridgeDishes, usePickerRows } from './usePickerRows';

export type GuestTarget = { group_id: string } | { new_group: NewGroup };

type Row = PickerRowContent & {
  key: string;
  icon?: ReactNode;
  danger?: boolean;
  ticked?: boolean;
  onSelect: () => void;
};

function GuestMenuBody({ group, guest, groups, onSelect, onVariation, onRemove, onSplit, onClose }: {
  group: GroupView | null;
  guest: PlannerGuest | null;
  groups: GroupView[];
  onSelect: (target: GuestTarget) => void;
  onVariation: (note: string | null) => void;
  onRemove: () => void;
  onSplit: () => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const [prompting, setPrompting] = useState(false);
  const { rows } = usePickerRows(query);
  const dishes = useFridgeDishes();
  const typed = query.trim();

  const thisMeal: Row[] = groups.map((candidate) => ({
    key: candidate.id,
    title: dishLabel(candidate),
    caption: null,
    concept: conceptFor(candidate),
    ticked: candidate.id === group?.id,
    onSelect: () => onSelect({ group_id: candidate.id }),
  }));
  const found: Row[] = typed ? [
    {
      key: 'label',
      title: `Add “${typed}”`,
      caption: 'Just a name, link a recipe later',
      concept: 'meal',
      onSelect: () => onSelect({ new_group: { label: typed } }),
    },
    ...rows.filter((row) => row.section === 'matches').map((row) => ({
      key: row.id,
      title: row.title,
      caption: row.caption,
      concept: row.concept,
      onSelect: () => onSelect({ new_group: row.group }),
    })),
  ] : [];
  const quick: Row[] = dishes.slice(0, 3).map((dish) => {
    const row = leftoversRow(dish);
    return { key: row.id, title: row.title, caption: row.caption, concept: row.concept, onSelect: () => onSelect({ new_group: row.group }) };
  });
  const actions: Row[] = guest ? [
    {
      key: 'variation',
      title: guest.note ? 'Change the variation' : 'Add a variation',
      caption: null,
      concept: 'meal',
      icon: <EditIcon sx={{ fontSize: 16 }} />,
      onSelect: () => setPrompting(true),
    },
    ...(guest.note ? [{
      key: 'clear-variation',
      title: 'Remove the variation',
      caption: null,
      concept: 'meal' as const,
      icon: <CloseIcon sx={{ fontSize: 16 }} />,
      danger: true,
      onSelect: () => onVariation(null),
    }] : []),
    {
      key: 'remove',
      title: 'Remove guest',
      caption: null,
      concept: 'meal',
      icon: <CloseIcon sx={{ fontSize: 16 }} />,
      danger: true,
      onSelect: onRemove,
    },
  ] : [];
  const visible = typed ? found : [...thisMeal, ...quick, ...actions];
  const current = Math.min(selected, Math.max(0, visible.length - 1));

  function keyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setSelected(Math.min(visible.length - 1, current + 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setSelected(Math.max(0, current - 1));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      visible[current]?.onSelect();
    } else if (event.key === 'Escape') {
      onClose();
    }
  }

  if (guest && guest.count > 1) return <Stack spacing={0.25} sx={{ p: 1 }}>
    <PickerRowButton row={{ title: `Split ${guest.count} guests`, caption: null, concept: 'meal' }} selected onPick={onSplit} />
  </Stack>;

  if (prompting && guest) return <PickerPromptStep
    label={`How ${guest.name ?? 'the guest'} has it`}
    initial={guest.note ?? ''}
    onSubmit={onVariation}
    onClose={onClose}
  />;

  return <Stack spacing={0.25} sx={{ p: 1 }}>
    <PickerSearchField value={query} onChange={(value) => { setQuery(value); setSelected(0); }} onKeyDown={keyDown}
      placeholder={`Something for ${guest?.name ?? 'guest'}`} />
    {typed ? found.map((row, index) => <PickerRowButton key={row.key} row={row} selected={index === current} onPick={row.onSelect} />) : <>
      {thisMeal.length > 0 ? <>
        <SectionHeading>This meal</SectionHeading>
        {thisMeal.map((row, index) => <PickerRowButton key={row.key} row={row} selected={index === current}
          trailing={row.ticked ? <CheckIcon sx={{ fontSize: 17, color: 'primary.main', display: 'block' }} /> : null} onPick={row.onSelect} />)}
      </> : null}
      {quick.length > 0 ? <>
        <SectionHeading>Quick picks</SectionHeading>
        {quick.map((row, index) => <PickerRowButton key={row.key} row={row} selected={thisMeal.length + index === current} onPick={row.onSelect} />)}
      </> : null}
      {actions.map((row, index) => <PickerRowButton key={row.key} row={row} selected={thisMeal.length + quick.length + index === current}
        icon={row.icon} danger={row.danger} onPick={row.onSelect} />)}
    </>}
  </Stack>;
}

export function GuestMenu({ open, anchorEl, sheet, group, guest, groups, onSelect, onVariation, onRemove, onSplit, onClose }: {
  open: boolean;
  anchorEl: HTMLElement | null;
  sheet: boolean;
  group: GroupView | null;
  guest: PlannerGuest | null;
  groups: GroupView[];
  onSelect: (target: GuestTarget) => void;
  onVariation: (note: string | null) => void;
  onRemove: () => void;
  onSplit: () => void;
  onClose: () => void;
}) {
  const body = open ? <GuestMenuBody key={guest?.id ?? 'new'} group={group} guest={guest} groups={groups} onSelect={onSelect}
    onVariation={onVariation} onRemove={onRemove} onSplit={onSplit} onClose={onClose} /> : null;
  if (sheet) return <Drawer anchor="bottom" open={open} onClose={onClose} slotProps={{ paper: { sx: { borderRadius: '14px 14px 0 0', maxHeight: '85vh' } } }}>{body}</Drawer>;
  return <Popover open={open && Boolean(anchorEl)} anchorEl={anchorEl} onClose={onClose} anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
    transformOrigin={{ vertical: 'top', horizontal: 'left' }} slotProps={{ paper: { sx: { width: 400, mt: 1, borderRadius: '14px', maxHeight: 'min(70vh, 560px)' } } }}>{body}</Popover>;
}
