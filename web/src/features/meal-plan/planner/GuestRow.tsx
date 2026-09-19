import AddIcon from '@mui/icons-material/AddOutlined';
import MoreVertIcon from '@mui/icons-material/MoreVertOutlined';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import InputBase from '@mui/material/InputBase';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { dishLabel } from './plannerWeek';
import type { GroupView, OccasionView, PlannerGuest } from './types';

const rowSx = {
  display: 'grid', gridTemplateColumns: '26px 1fr auto 28px', gap: 1.5,
  alignItems: 'center', py: 1.5, borderTop: '1px solid', borderColor: 'divider',
} as const;

function GuestAvatar() {
  return <Box aria-hidden sx={{ width: 26, height: 26, borderRadius: '50%', border: '1px dashed', borderColor: 'divider' }} />;
}

function GuestName({ guest, label, onRename }: { guest: PlannerGuest; label: string; onRename: (guest: PlannerGuest, name: string | null) => void }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState('');

  function save() {
    if (!editing) return;
    setEditing(false);
    const next = name.trim() || null;
    if (next !== guest.name) onRename(guest, next);
  }

  return editing ? <InputBase autoFocus value={name} inputProps={{ 'aria-label': `Name for ${label}` }} onChange={(event) => setName(event.target.value)}
      onBlur={save} onKeyDown={(event) => { if (event.key === 'Enter') save(); if (event.key === 'Escape') setEditing(false); }}
      sx={{ width: '100%', border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1 }} />
    : <Typography component="div" sx={{ fontWeight: 500, lineHeight: 1.25, minWidth: 0 }} noWrap>
      <Box component="button" type="button" aria-label={`Edit name for ${label}`} onClick={() => { setName(guest.name ?? ''); setEditing(true); }}
        sx={{ p: 0, border: 0, background: 'none', color: 'text.primary', font: 'inherit', fontWeight: 'inherit', textAlign: 'left', cursor: 'text' }}>{label}</Box>
      {guest.note ? <Typography component="span" variant="body2" color="text.secondary" sx={{ ml: 0.5 }}>· {guest.note}</Typography> : null}
    </Typography>;
}

export function GuestRows({ occasion, onAdd, onOpen, onRename }: {
  occasion: OccasionView;
  onAdd: (anchor: HTMLElement) => void;
  onOpen: (group: GroupView, guest: PlannerGuest, anchor: HTMLElement) => void;
  onRename: (guest: PlannerGuest, name: string | null) => void;
}) {
  const guests = occasion.groups.flatMap((group) => group.guests.map((guest) => ({ group, guest })));
  return (
    <>
      {guests.map(({ group, guest }, index) => {
        const label = guest.name ?? (guest.count > 1 ? `${guest.count} guests` : `Guest ${index + 1}`);
        return <Box key={guest.id} sx={rowSx}>
          <GuestAvatar />
          {guest.count > 1 ? <Typography sx={{ color: 'text.secondary' }}>{label}</Typography> : <GuestName guest={guest} label={label} onRename={onRename} />}
          <Typography variant="body2" sx={{ justifySelf: 'end', textAlign: 'right' }}>{dishLabel(group)}</Typography>
          <IconButton size="small" aria-label={`Change ${label}`} onClick={(event) => onOpen(group, guest, event.currentTarget)} sx={{ color: 'text.disabled' }}>
            <MoreVertIcon sx={{ fontSize: 18 }} />
          </IconButton>
        </Box>;
      })}
      <Box sx={rowSx}>
        <GuestAvatar />
        <Typography sx={{ color: 'text.secondary' }}>Add a guest</Typography>
        <Box />
        <IconButton size="small" aria-label="Add a guest" onClick={(event) => onAdd(event.currentTarget)} sx={{ color: 'text.disabled' }}>
          <AddIcon sx={{ fontSize: 16 }} />
        </IconButton>
      </Box>
    </>
  );
}
