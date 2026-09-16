import CheckIcon from '@mui/icons-material/CheckOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import Box from '@mui/material/Box';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type ReactNode } from 'react';
import type { MealSlot } from '../../api/client';
import { labelForSlot } from './slots';

export function OptionCard({
  title,
  caption,
  icon,
  selected,
  disabled,
  onClick,
}: {
  title: string;
  caption?: string;
  icon?: ReactNode;
  selected: boolean;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <ButtonBase
      onClick={onClick}
      disabled={disabled}
      aria-pressed={disabled ? undefined : selected}
      aria-disabled={disabled ? true : undefined}
      sx={{ display: 'block', width: '100%', textAlign: 'left', borderRadius: '14px' }}
    >
      <Paper
        elevation={0}
        sx={{
          width: '100%',
          p: 2,
          borderColor: selected ? 'primary.main' : 'divider',
          ...(disabled ? { borderStyle: 'dashed', color: 'text.disabled' } : {}),
        }}
      >
        <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
          <Box sx={{ width: 34, height: 34, borderRadius: '10px', bgcolor: 'action.hover', display: 'grid', placeItems: 'center', color: disabled ? 'text.disabled' : 'text.secondary' }}>
            {selected ? <CheckIcon /> : icon}
          </Box>
          <Stack spacing={0.5}>
            <Typography variant="body1" sx={{ fontWeight: 500 }}>{title}</Typography>
            {caption ? <Typography variant="caption" color={disabled ? 'text.disabled' : 'text.secondary'}>{caption}</Typography> : null}
          </Stack>
        </Stack>
      </Paper>
    </ButtonBase>
  );
}

export function MealTimeChip({
  slot,
  time,
  onChange,
}: {
  slot: MealSlot;
  time: string;
  onChange: (value: string) => void;
}) {
  const [editing, setEditing] = useState(false);

  if (editing) {
    return (
      <TextField
        autoFocus
        aria-label={slot === 'snacks' ? 'Time (optional)' : 'Time'}
        type="time"
        size="small"
        value={time}
        onChange={(event) => onChange(event.target.value)}
        onBlur={() => setEditing(false)}
        slotProps={{ htmlInput: { style: { paddingTop: 6, paddingBottom: 6 } } }}
        sx={{ width: 128 }}
      />
    );
  }

  return (
    <Chip
      size="small"
      icon={<EditIcon />}
      label={`${labelForSlot(slot)} · ${time || 'Add time'}`}
      onClick={() => setEditing(true)}
      sx={{ borderRadius: 999, bgcolor: 'action.hover', border: 0 }}
    />
  );
}
