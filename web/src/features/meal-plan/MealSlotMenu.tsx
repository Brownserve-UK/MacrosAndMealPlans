import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import { useState } from 'react';
import type { MealSlot } from '../../api/client';

export function MealSlotMenu({
  choices,
  onSelect,
  label = 'Plan meal',
  variant = 'contained',
}: {
  choices: { value: MealSlot; label: string }[];
  onSelect: (slot: MealSlot) => void;
  label?: string;
  variant?: 'contained' | 'text';
}) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  return (
    <>
      <Button
        variant={variant}
        size={variant === 'text' ? 'small' : 'medium'}
        startIcon={variant === 'contained' ? <AddIcon /> : undefined}
        onClick={(event) => setAnchor(event.currentTarget)}
      >
        {label}
      </Button>
      <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
        {choices.map((choice) => (
          <MenuItem
            key={choice.value}
            onClick={() => {
              setAnchor(null);
              onSelect(choice.value);
            }}
          >
            {choice.label}
          </MenuItem>
        ))}
      </Menu>
    </>
  );
}
