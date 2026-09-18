import AddIcon from '@mui/icons-material/AddOutlined';
import LunchDiningIcon from '@mui/icons-material/LunchDiningOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlined';
import StorefrontIcon from '@mui/icons-material/StorefrontOutlined';
import Box from '@mui/material/Box';
import ButtonBase from '@mui/material/ButtonBase';
import Drawer from '@mui/material/Drawer';
import InputBase from '@mui/material/InputBase';
import Popover from '@mui/material/Popover';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { alpha } from '@mui/material/styles';
import type { KeyboardEvent, ReactNode } from 'react';
import { useMemo, useState } from 'react';
import { IconTile } from '../../../components/IconTile';
import type { NewGroup, PickerRow } from './types';
import { usePickerRows } from './usePickerRows';

function TileFor({ concept }: { concept: PickerRow['concept'] }) {
  const boxed = (icon: ReactNode) => (
    <Box
      sx={{
        width: 34,
        height: 34,
        flexShrink: 0,
        borderRadius: '10px',
        display: 'grid',
        placeItems: 'center',
        backgroundColor: 'background.default',
        color: 'text.secondary',
      }}
    >
      {icon}
    </Box>
  );
  switch (concept) {
    case 'recipe':
      return <IconTile concept="recipe" tone="primary" />;
    case 'saved_meal':
      return <IconTile concept="meal" tone="primary" />;
    case 'food':
      return <IconTile concept="food" />;
    case 'dish':
      return <IconTile concept="dish" tone="secondary" />;
    case 'out':
      return boxed(<StorefrontIcon sx={{ fontSize: 17 }} />);
    case 'takeaway':
      return boxed(<LunchDiningIcon sx={{ fontSize: 17 }} />);
    case 'fend':
      return boxed(<PersonIcon sx={{ fontSize: 17 }} />);
    default:
      return boxed(<AddIcon sx={{ fontSize: 17 }} />);
  }
}

export function PickerRowButton({
  row,
  selected,
  trailing,
  onPick,
}: {
  row: PickerRow;
  selected: boolean;
  trailing?: ReactNode;
  onPick: (anchor: HTMLElement) => void;
}) {
  return (
    <ButtonBase
      onClick={(event) => onPick(event.currentTarget)}
      sx={{
        display: 'grid',
        gridTemplateColumns: '34px 1fr auto',
        gap: 1.5,
        alignItems: 'center',
        width: '100%',
        px: 1.25,
        py: 0.75,
        borderRadius: '10px',
        textAlign: 'left',
        backgroundColor: selected ? 'action.hover' : 'transparent',
        '&:hover': { backgroundColor: 'action.hover' },
      }}
    >
      <TileFor concept={row.concept} />
      <Box sx={{ minWidth: 0 }}>
        <Typography sx={{ fontWeight: 500, lineHeight: 1.25 }}>{row.title}</Typography>
        {row.caption ? (
          <Typography variant="caption" color="text.secondary" className="numeral">
            {row.caption}
          </Typography>
        ) : null}
      </Box>
      <Box>{trailing}</Box>
    </ButtonBase>
  );
}

function PickerBody({
  initialQuery,
  placeholder,
  onPick,
  onClose,
}: {
  initialQuery: string;
  placeholder: string;
  onPick: (group: NewGroup) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState(initialQuery);
  const [selected, setSelected] = useState(0);
  const { rows, loading } = usePickerRows(query);

  const all = useMemo<PickerRow[]>(() => {
    const typed = query.trim();
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
    return [...head, ...rows];
  }, [query, rows]);

  const current = Math.min(selected, Math.max(0, all.length - 1));

  function keyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setSelected(Math.min(all.length - 1, current + 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setSelected(Math.max(0, current - 1));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const row = all[current];
      if (row) onPick(row.group);
    } else if (event.key === 'Escape') {
      onClose();
    }
  }

  const matches = all.filter((row) => row.section === 'matches');
  const quick = all.filter((row) => row.section === 'quick');

  return (
    <Stack spacing={0.25} sx={{ p: 1 }}>
      <InputBase
        autoFocus
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          setSelected(0);
        }}
        onKeyDown={keyDown}
        placeholder={placeholder}
        inputProps={{ 'aria-label': placeholder }}
        sx={(theme) => ({
          px: 1.5,
          py: 1,
          mb: 0.5,
          borderRadius: '10px',
          border: '1px solid',
          borderColor: 'primary.main',
          boxShadow: theme.vars
            ? `0 0 0 3px rgba(${theme.vars.palette.primary.mainChannel} / 0.12)`
            : `0 0 0 3px ${alpha(theme.palette.primary.main, 0.12)}`,
          fontWeight: 500,
        })}
      />
      {matches.map((row) => (
        <PickerRowButton
          key={row.id}
          row={row}
          selected={all.indexOf(row) === current}
          trailing={
            row.id === 'label' ? (
              <Typography
                variant="caption"
                sx={{ color: 'text.disabled', border: '1px solid', borderColor: 'divider', borderRadius: '6px', px: 0.75 }}
              >
                ↵
              </Typography>
            ) : null
          }
          onPick={() => onPick(row.group)}
        />
      ))}
      {matches.length === 0 && !loading && query.trim() === '' ? (
        <Typography variant="body2" color="text.secondary" sx={{ px: 1.25, py: 1 }}>
          Type a meal, or pick something below.
        </Typography>
      ) : null}
      <Typography
        variant="caption"
        sx={{ px: 1.25, pt: 1.5, pb: 0.5, letterSpacing: '0.06em', textTransform: 'uppercase', fontWeight: 600, color: 'text.secondary' }}
      >
        Quick picks
      </Typography>
      {quick.map((row) => (
        <PickerRowButton
          key={row.id}
          row={row}
          selected={all.indexOf(row) === current}
          onPick={() => onPick(row.group)}
        />
      ))}
    </Stack>
  );
}

export function AddMealPicker({
  open,
  anchorEl,
  sheet,
  initialQuery,
  placeholder = 'What are you eating?',
  onPick,
  onClose,
}: {
  open: boolean;
  anchorEl: HTMLElement | null;
  sheet: boolean;
  initialQuery: string;
  placeholder?: string;
  onPick: (group: NewGroup) => void;
  onClose: () => void;
}) {
  if (sheet) {
    return (
      <Drawer anchor="bottom" open={open} onClose={onClose} slotProps={{ paper: { sx: { borderRadius: '14px 14px 0 0', maxHeight: '85vh' } } }}>
        {open ? <PickerBody initialQuery={initialQuery} placeholder={placeholder} onPick={onPick} onClose={onClose} /> : null}
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
      slotProps={{ paper: { sx: { width: 400, mt: 1, borderRadius: '14px' } } }}
    >
      {open ? <PickerBody initialQuery={initialQuery} placeholder={placeholder} onPick={onPick} onClose={onClose} /> : null}
    </Popover>
  );
}
