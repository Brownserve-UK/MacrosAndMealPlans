import Box from '@mui/material/Box';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import { useNavigate } from '@tanstack/react-router';
import { defaultDayFor } from './date';

export type Lens = 'mine' | 'household';

export function PlannerLens({
  lens,
  weekStart,
  day,
  show,
}: {
  lens: Lens;
  weekStart: string;
  day: string;
  show: boolean;
}) {
  const navigate = useNavigate();
  if (!show) return null;

  function change(next: Lens | null) {
    if (!next || next === lens) return;
    const params = { weekStart, day: day || defaultDayFor(weekStart) };
    void navigate({
      to: next === 'mine' ? '/planner/$weekStart/$day' : '/household/planner/$weekStart/$day',
      params,
    });
  }

  return (
    <Box>
      <ToggleButtonGroup
        exclusive
        size="small"
        value={lens}
        onChange={(_, next: Lens | null) => change(next)}
        sx={{
          bgcolor: 'action.hover',
          borderRadius: '10px',
          p: 0.375,
          '& .MuiToggleButton-root': {
            border: 'none',
            borderRadius: '8px !important',
            px: 1.75,
            py: 0.75,
            textTransform: 'none',
            fontWeight: 600,
            color: 'text.secondary',
          },
          '& .Mui-selected': {
            bgcolor: 'background.paper',
            color: 'text.primary',
          },
        }}
      >
        <ToggleButton value="mine">Mine</ToggleButton>
        <ToggleButton value="household">Household</ToggleButton>
      </ToggleButtonGroup>
    </Box>
  );
}
