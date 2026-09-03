import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import { useState } from 'react';
import type { MealSlot } from '../../api/client';

export function MealSlotMenu({
  choices,
  onSelect,
}: {
  choices: { value: MealSlot; label: string }[];
  onSelect: (slot: MealSlot) => void;
}) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  return (
    <>
      <Button variant="contained" startIcon={<AddIcon />} onClick={(event) => setAnchor(event.currentTarget)}>
        Plan meal
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
