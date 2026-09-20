import AddIcon from '@mui/icons-material/AddOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import MoreVertIcon from '@mui/icons-material/MoreVertOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlined';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import InputBase from '@mui/material/InputBase';
import Typography from '@mui/material/Typography';
import { useRef, useState } from 'react';
import { DinerMeal, dinerRowSx } from './DinerRow';
import type { GroupView, OccasionView, PlannerGuest } from './types';

function GuestAvatar({ placeholder = false }: { placeholder?: boolean }) {
  return <Box aria-hidden sx={{
    width: 26, height: 26, borderRadius: '50%', display: 'grid', placeItems: 'center',
    backgroundColor: placeholder ? 'transparent' : 'divider',
    border: placeholder ? '1px dashed' : 'none',
    borderColor: placeholder ? 'text.disabled' : undefined,
    color: placeholder ? 'text.disabled' : 'text.secondary',
  }}>
    <PersonIcon sx={{ fontSize: 16 }} />
  </Box>;
}

function GuestName({ guest, label, onRename }: { guest: PlannerGuest; label: string; onRename: (guest: PlannerGuest, name: string | null) => Promise<boolean> }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const active = useRef(false);
  const changed = useRef(false);
  const button = useRef<HTMLButtonElement>(null);

  async function save(restoreFocus = false) {
    if (!active.current) return;
    active.current = false;
    const next = name.trim() || null;
    if (changed.current && next !== guest.name) {
      setSaving(true);
      const saved = await onRename(guest, next);
      setSaving(false);
      if (!saved) { active.current = true; return; }
    }
    setEditing(false);
    if (restoreFocus) requestAnimationFrame(() => button.current?.focus());
  }

  return <Typography component="div" sx={{ fontWeight: 500, lineHeight: 1.25, minWidth: 0, overflow: 'hidden', whiteSpace: 'nowrap' }}>
      <Box component="span" sx={{ display: 'inline-grid', gridTemplateColumns: 'minmax(0, 1fr) auto', alignItems: 'center', maxWidth: '100%', verticalAlign: 'bottom', columnGap: 0.5 }}>
        {editing ? <Box sx={{ display: 'grid', minWidth: 0, position: 'relative' }}>
          <Box component="span" aria-hidden sx={{ gridArea: '1 / 1', visibility: 'hidden', whiteSpace: 'pre', overflow: 'hidden' }}>{name || label}</Box>
          <InputBase autoFocus value={name} readOnly={saving} onFocus={(event) => event.target.select()}
            inputProps={{ 'aria-label': `Name for ${label}`, 'aria-busy': saving }}
            onChange={(event) => { changed.current = true; setName(event.target.value); }}
            onBlur={() => { void save(); }} onKeyDown={(event) => {
              if (event.nativeEvent.isComposing) return;
              if (event.key === 'Enter') { event.preventDefault(); void save(true); }
              if (event.key === 'Escape' && !saving) { event.preventDefault(); active.current = false; setEditing(false); requestAnimationFrame(() => button.current?.focus()); }
            }}
            sx={{ position: 'absolute', inset: 0, width: '100%', minWidth: 0, font: 'inherit', color: 'inherit', '& input': { p: 0, height: 'auto', font: 'inherit' } }} />
        </Box> : <Box ref={button} component="button" type="button" aria-label={`Edit name for ${label}`}
          onClick={() => { setName(label); changed.current = false; active.current = true; setEditing(true); }}
          sx={{ gridColumn: '1 / 3', display: 'flex', alignItems: 'center', gap: 0.5, minWidth: 0, p: 0, border: 0, background: 'none', color: 'text.primary', font: 'inherit', textAlign: 'left', cursor: 'text', '&:focus-visible': { outline: '1px solid', outlineColor: 'primary.main' }, '&:hover svg, &:focus-visible svg': { color: 'text.primary' } }}>
          <Box component="span" sx={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</Box>
          <EditIcon sx={{ fontSize: 'inherit', color: 'text.secondary', flexShrink: 0 }} />
        </Box>}
        {editing ? <EditIcon aria-hidden sx={{ fontSize: 'inherit', visibility: 'hidden' }} /> : null}
      </Box>
      {guest.note ? <Typography component="span" variant="body2" color="text.secondary" sx={{ ml: 0.5 }}>· {guest.note}</Typography> : null}
    </Typography>;
}

export function GuestRows({ occasion, onAdd, onOpen, onRename }: {
  occasion: OccasionView;
  onAdd: (anchor: HTMLElement) => void;
  onOpen: (group: GroupView, guest: PlannerGuest, anchor: HTMLElement) => void;
  onRename: (guest: PlannerGuest, name: string | null) => Promise<boolean>;
}) {
  const guests = occasion.groups.flatMap((group) => group.guests.map((guest) => ({ group, guest })));
  return (
    <>
      {guests.map(({ group, guest }, index) => {
        const label = guest.name ?? (guest.count > 1 ? `${guest.count} guests` : `Guest ${index + 1}`);
        return <Box key={guest.id} sx={dinerRowSx}>
          <GuestAvatar />
          {guest.count > 1 ? <Typography sx={{ color: 'text.secondary' }}>{label}</Typography> : <GuestName guest={guest} label={label} onRename={onRename} />}
          <DinerMeal group={group} />
          <IconButton size="small" aria-label={`Change ${label}`} onClick={(event) => onOpen(group, guest, event.currentTarget)} sx={{ color: 'text.disabled' }}>
            <MoreVertIcon sx={{ fontSize: 18 }} />
          </IconButton>
        </Box>;
      })}
      <Box sx={dinerRowSx}>
        <GuestAvatar placeholder />
        <Typography sx={{ color: 'text.secondary' }}>Add a guest</Typography>
        <Box />
        <IconButton size="small" aria-label="Add a guest" onClick={(event) => onAdd(event.currentTarget)} sx={{ color: 'text.disabled' }}>
          <AddIcon sx={{ fontSize: 16 }} />
        </IconButton>
      </Box>
    </>
  );
}
