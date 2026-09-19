import WarningIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import type { Amount } from '../../api/client';
import { formatAmount } from './format';

export function Fact({ icon, label, value }: { icon?: ReactNode; label: string; value: string }) {
  return (
    <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
      {icon ? <Box sx={{ color: 'warning.main', display: 'grid', placeItems: 'center' }}>{icon}</Box> : null}
      <Stack spacing={0}>
        <Typography variant="caption" sx={{ color: 'text.secondary', fontWeight: 650, lineHeight: 1.2, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          {label}
        </Typography>
        <Typography variant="body2" className="numeral" sx={{ color: 'text.primary', lineHeight: 1.35 }}>
          {value}
        </Typography>
      </Stack>
    </Stack>
  );
}

export function FactBar({ children }: { children: ReactNode }) {
  return (
    <Stack
      direction="row"
      spacing={{ xs: 2, sm: 2.5 }}
      useFlexGap
      divider={<Divider orientation="vertical" flexItem sx={{ display: { xs: 'none', sm: 'block' } }} />}
      sx={{ flexWrap: 'wrap', alignItems: 'center' }}
    >
      {children}
    </Stack>
  );
}

export type MealCardFood = { id: string; name: string; amount: Amount };

export function MealCard({
  header,
  foods,
  warning,
  actions,
}: {
  header: ReactNode;
  foods: MealCardFood[];
  warning?: string | null;
  actions?: ReactNode;
}) {
  return (
    <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
      <Box sx={{ px: { xs: 2, sm: 2.5 }, py: 1.75 }}>{header}</Box>
      <Divider />
      <Stack divider={<Divider flexItem />}>
        {foods.map((food) => (
          <Stack
            key={food.id}
            direction="row"
            spacing={2}
            sx={{ justifyContent: 'space-between', px: { xs: 2, sm: 2.5 }, py: 1.5 }}
          >
            <Typography>{food.name}</Typography>
            <Typography color="text.secondary" sx={{ whiteSpace: 'nowrap' }}>
              {formatAmount(food.amount)}
            </Typography>
          </Stack>
        ))}
      </Stack>
      {warning ? (
        <Stack
          direction="row"
          spacing={1}
          sx={{ alignItems: 'center', px: { xs: 2, sm: 2.5 }, py: 1.25, color: 'warning.dark', bgcolor: 'warning.50' }}
        >
          <WarningIcon fontSize="small" />
          <Typography variant="body2">{warning}</Typography>
        </Stack>
      ) : null}
      {actions ? (
        <Stack direction="row" spacing={1} sx={{ px: { xs: 1.5, sm: 2 }, py: 1.25, flexWrap: 'wrap', borderTop: '1px solid', borderColor: 'divider' }}>
          {actions}
        </Stack>
      ) : null}
    </Paper>
  );
}
