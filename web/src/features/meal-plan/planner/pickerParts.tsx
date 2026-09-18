import AddIcon from '@mui/icons-material/AddOutlined';
import LunchDiningIcon from '@mui/icons-material/LunchDiningOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlined';
import StorefrontIcon from '@mui/icons-material/StorefrontOutlined';
import Box from '@mui/material/Box';
import ButtonBase from '@mui/material/ButtonBase';
import InputBase from '@mui/material/InputBase';
import Typography from '@mui/material/Typography';
import { alpha } from '@mui/material/styles';
import type { KeyboardEvent, ReactNode } from 'react';
import { IconTile } from '../../../components/IconTile';
import type { PickerRow } from './types';

export function BoxedTile({ icon, color }: { icon: ReactNode; color?: string }) {
  return (
    <Box
      sx={{
        width: 34,
        height: 34,
        flexShrink: 0,
        borderRadius: '10px',
        display: 'grid',
        placeItems: 'center',
        backgroundColor: 'background.default',
        color: color ?? 'text.secondary',
      }}
    >
      {icon}
    </Box>
  );
}

export function TileFor({ concept }: { concept: PickerRow['concept'] }) {
  const boxed = (icon: ReactNode) => <BoxedTile icon={icon} />;
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

export function SectionHeading({ children }: { children: ReactNode }) {
  return (
    <Typography
      variant="caption"
      sx={{ px: 1.25, pt: 1.5, pb: 0.5, letterSpacing: '0.06em', textTransform: 'uppercase', fontWeight: 600, color: 'text.secondary' }}
    >
      {children}
    </Typography>
  );
}

export type PickerRowContent = {
  title: string;
  caption?: string | null;
  concept: PickerRow['concept'];
};

export function PickerRowButton({
  row,
  selected,
  trailing,
  icon,
  danger,
  onPick,
}: {
  row: PickerRowContent;
  selected: boolean;
  trailing?: ReactNode;
  icon?: ReactNode;
  danger?: boolean;
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
      {icon ? <BoxedTile icon={icon} color={danger ? 'error.main' : undefined} /> : <TileFor concept={row.concept} />}
      <Box sx={{ minWidth: 0 }}>
        <Typography sx={{ fontWeight: 500, lineHeight: 1.25, color: danger ? 'error.main' : undefined }}>{row.title}</Typography>
        {row.caption ? (
          <Typography variant="caption" color="text.secondary" className="numeral" sx={{ display: 'block' }}>
            {row.caption}
          </Typography>
        ) : null}
      </Box>
      <Box>{trailing}</Box>
    </ButtonBase>
  );
}

export function PickerSearchField({
  value,
  onChange,
  onKeyDown,
  placeholder,
}: {
  value: string;
  onChange: (value: string) => void;
  onKeyDown: (event: KeyboardEvent<HTMLInputElement>) => void;
  placeholder: string;
}) {
  return (
    <InputBase
      autoFocus
      value={value}
      onChange={(event) => onChange(event.target.value)}
      onKeyDown={onKeyDown}
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
  );
}
