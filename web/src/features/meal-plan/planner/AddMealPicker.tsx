import Drawer from '@mui/material/Drawer';
import Popover from '@mui/material/Popover';
import Stack from '@mui/material/Stack';
import type { KeyboardEvent } from 'react';
import { useMemo, useState } from 'react';
import { PickerRowButton, PickerSearchField, SectionHeading } from './pickerParts';
import type { NewGroup, PickerRow } from './types';
import { usePickerRows } from './usePickerRows';

function PickerBody({
  plannedOn,
  initialQuery,
  placeholder,
  onPick,
  onClose,
}: {
  plannedOn: string;
  initialQuery: string;
  placeholder: string;
  onPick: (group: NewGroup) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState(initialQuery);
  const [selected, setSelected] = useState(0);
  const { rows } = usePickerRows(query, plannedOn);

  const typed = query.trim();

  const found = useMemo<PickerRow[]>(() => {
    const head: PickerRow[] = typed
      ? [
          {
            id: 'label',
            title: `Add “${typed}”`,
            caption: 'Just a name, link a recipe later',
            concept: 'meal',
            section: 'matches',
            group: { label: typed },
          },
        ]
      : [];
    return [...head, ...rows.filter((row) => row.section === 'matches')];
  }, [rows, typed]);

  const quick = useMemo(() => (typed ? [] : rows.filter((row) => row.section === 'quick')), [rows, typed]);
  const flat = useMemo(() => [...found, ...quick], [found, quick]);
  const current = Math.min(selected, Math.max(0, flat.length - 1));

  function keyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setSelected(Math.min(flat.length - 1, current + 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setSelected(Math.max(0, current - 1));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const row = flat[current];
      if (row) onPick(row.group);
    } else if (event.key === 'Escape') {
      onClose();
    }
  }

  return (
    <Stack spacing={0.25} sx={{ p: 1 }}>
      <PickerSearchField
        value={query}
        onChange={(value) => {
          setQuery(value);
          setSelected(0);
        }}
        onKeyDown={keyDown}
        placeholder={placeholder}
      />

      {found.map((row, index) => (
        <PickerRowButton key={row.id} row={row} selected={index === current} onPick={() => onPick(row.group)} />
      ))}

      {quick.length > 0 ? (
        <>
          <SectionHeading>Quick picks</SectionHeading>
          {quick.map((row, index) => (
            <PickerRowButton
              key={row.id}
              row={row}
              selected={found.length + index === current}
              onPick={() => onPick(row.group)}
            />
          ))}
        </>
      ) : null}
    </Stack>
  );
}

export function AddMealPicker({
  plannedOn,
  open,
  anchorEl,
  sheet,
  initialQuery,
  placeholder = 'What are you eating?',
  onPick,
  onClose,
}: {
  plannedOn: string;
  open: boolean;
  anchorEl: HTMLElement | null;
  sheet: boolean;
  initialQuery: string;
  placeholder?: string;
  onPick: (group: NewGroup) => void;
  onClose: () => void;
}) {
  const body = open ? <PickerBody plannedOn={plannedOn} initialQuery={initialQuery} placeholder={placeholder} onPick={onPick} onClose={onClose} /> : null;

  if (sheet) {
    return (
      <Drawer anchor="bottom" open={open} onClose={onClose} slotProps={{ paper: { sx: { borderRadius: '14px 14px 0 0', maxHeight: '85vh' } } }}>
        {body}
      </Drawer>
    );
  }
  return (
    <Popover
      open={open && Boolean(anchorEl)}
      anchorEl={anchorEl}
      onClose={onClose}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      slotProps={{ paper: { sx: { width: 400, mt: 1, borderRadius: '14px', maxHeight: 'min(70vh, 560px)' } } }}
    >
      {body}
    </Popover>
  );
}
