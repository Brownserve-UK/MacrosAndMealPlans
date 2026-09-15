import ArrowBackIcon from '@mui/icons-material/ArrowBackOutlined';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';

export function BackLabel({ children }: { children: string }) {
  return (
    <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', color: 'text.secondary' }}>
      <ArrowBackIcon sx={{ fontSize: 16 }} />
      <Typography variant="body2">{children}</Typography>
    </Stack>
  );
}
