import Dialog, { type DialogProps } from '@mui/material/Dialog';
import { DRAWER_WIDTH } from './AppShell';

export function FormDialog({ sx, ...props }: DialogProps) {
  return (
    <Dialog
      {...props}
      sx={[
        { '& .MuiDialog-container': { pl: { md: `${DRAWER_WIDTH}px` } } },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
    />
  );
}
