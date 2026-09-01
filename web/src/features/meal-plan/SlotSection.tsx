import AddIcon from '@mui/icons-material/AddOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';

export function SlotSection({
  id,
  title,
  action,
  children,
}: {
  id: string;
  title: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Box component="section" aria-labelledby={`slot-${id}`}>
      <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
        <Typography variant="h3" id={`slot-${id}`}>
          {title}
        </Typography>
        {action}
      </Stack>
      {children}
    </Box>
  );
}

export function EmptySlot({
  label,
  onClick,
  disabled,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <Button
      fullWidth
      startIcon={<AddIcon />}
      onClick={onClick}
      disabled={disabled}
      sx={{
        justifyContent: 'center',
        py: 1.5,
        borderRadius: 1.5,
        border: '1px dashed',
        borderColor: 'divider',
        color: 'primary.main',
        '&:hover': { borderColor: 'primary.main', bgcolor: 'action.hover' },
      }}
    >
      {label}
    </Button>
  );
}
