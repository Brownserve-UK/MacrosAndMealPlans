import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { alpha } from '@mui/material/styles';
import { PickerRowButton } from './AddMealPicker';
import type { PlannerMember, PickerRow } from './types';
import { leftoversRow, servingsInFridge, type FridgeDish } from './usePickerRows';

const SAVED_ROW: PickerRow = {
  id: 'saved',
  title: 'A saved meal',
  caption: 'Pick from saved meals',
  concept: 'saved_meal',
  section: 'matches',
  group: {},
};

const ELSE_ROW: PickerRow = {
  id: 'else',
  title: 'Something else',
  caption: 'Type a name',
  concept: 'meal',
  section: 'matches',
  group: {},
};

const OUT_ROW: PickerRow = {
  id: 'out',
  title: 'Eating elsewhere',
  caption: null,
  concept: 'out',
  section: 'matches',
  group: {},
};

export function NeedsMealSlab({
  member,
  dishes,
  onLeftovers,
  onSavedMeal,
  onSomethingElse,
  onElsewhere,
}: {
  member: PlannerMember;
  dishes: FridgeDish[];
  onLeftovers: (row: PickerRow) => void;
  onSavedMeal: (anchor: HTMLElement) => void;
  onSomethingElse: (anchor: HTMLElement) => void;
  onElsewhere: () => void;
}) {
  return (
    <Box sx={(theme) => ({ p: 1.75, borderRadius: '10px', backgroundColor: alpha(theme.palette.warning.main, 0.09) })}>
      <Typography sx={{ fontWeight: 600, color: 'warning.main', mb: 0.75 }}>{`${member.name} needs a meal`}</Typography>
      <Stack spacing={0.25}>
        {dishes.slice(0, 2).map((dish) => {
          const row = { ...leftoversRow(dish, 1), caption: `${dish.name}, ${servingsInFridge(dish)}` };
          return <PickerRowButton key={row.id} row={row} selected={false} onPick={() => onLeftovers(row)} />;
        })}
        <PickerRowButton row={SAVED_ROW} selected={false} onPick={onSavedMeal} />
        <PickerRowButton row={ELSE_ROW} selected={false} onPick={onSomethingElse} />
        <PickerRowButton row={OUT_ROW} selected={false} onPick={onElsewhere} />
      </Stack>
    </Box>
  );
}
