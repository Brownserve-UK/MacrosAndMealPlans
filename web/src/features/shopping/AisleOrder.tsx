import ArrowDownIcon from '@mui/icons-material/ArrowDownwardOutlined';
import ArrowUpIcon from '@mui/icons-material/ArrowUpwardOutlined';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ShoppingSection } from '../../api/client';
import { useHouseholdSettings, useUpdateHouseholdSettings } from '../../api/queries';
import { sectionLabel } from './sections';

export function AisleOrder() {
  const settings = useHouseholdSettings();
  const save = useUpdateHouseholdSettings();

  if (settings.isLoading) {
    return (
      <Typography variant="body2" color="text.secondary">
        Loading aisle order…
      </Typography>
    );
  }
  const order = settings.data?.shopping_section_order;
  if (!order) return null;

  function moveTo(from: number, to: number) {
    if (!order || !settings.data || to < 0 || to >= order.length) return;
    const next = [...order];
    const [held] = next.splice(from, 1);
    next.splice(to, 0, held as ShoppingSection);
    save.mutate({
      revision: settings.data.revision,
      body: { shopping_section_order: next },
    });
  }

  return (
    <Stack spacing={1}>
      <Typography variant="h3">Aisle order</Typography>
      <Typography variant="body2" color="text.secondary">
        Match your store, so you walk it once.
      </Typography>

      <Paper variant="outlined" sx={{ overflow: 'hidden', mt: 1 }}>
        {order.map((section, index) => (
          <Stack
            key={section}
            direction="row"
            spacing={1}
            sx={{
              alignItems: 'center',
              px: 2,
              py: 0.75,
              borderTop: index === 0 ? 0 : 1,
              borderColor: 'divider',
            }}
          >
            <Typography variant="body1" sx={{ flexGrow: 1 }}>
              {sectionLabel(section)}
            </Typography>
            <IconButton
              size="small"
              aria-label={`Move ${sectionLabel(section)} up`}
              disabled={index === 0 || save.isPending}
              onClick={() => moveTo(index, index - 1)}
            >
              <ArrowUpIcon fontSize="small" />
            </IconButton>
            <IconButton
              size="small"
              aria-label={`Move ${sectionLabel(section)} down`}
              disabled={index === order.length - 1 || save.isPending}
              onClick={() => moveTo(index, index + 1)}
            >
              <ArrowDownIcon fontSize="small" />
            </IconButton>
          </Stack>
        ))}
      </Paper>
    </Stack>
  );
}
