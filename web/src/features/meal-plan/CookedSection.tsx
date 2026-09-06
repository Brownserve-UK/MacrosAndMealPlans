import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useCooks } from '../../api/queries';
import { IconTile } from '../../components/IconTile';
import { SlotSection } from './SlotSection';

function timeOf(preparedAt: string): string {
  const at = new Date(preparedAt);
  return Number.isNaN(at.getTime())
    ? ''
    : at.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
}

export function CookedSection({ date }: { date: string }) {
  const cooks = useCooks(date, date);
  const standalone = (cooks.data ?? []).filter((cook) => !cook.meal_plan_component_id);
  if (standalone.length === 0) return null;

  return (
    <SlotSection id="cooked" title="Cooked">
      <Stack spacing={1.5}>
        {standalone.map((cook) => (
          <Paper key={cook.id} sx={{ px: 2.25, py: 1.75 }}>
            <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
              <IconTile concept="cook" tone="secondary" />
              <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
                <Stack direction="row" spacing={1.25} sx={{ alignItems: 'baseline' }}>
                  <Typography variant="caption" color="text.secondary" className="numeral" sx={{ flexShrink: 0 }}>
                    {timeOf(cook.prepared_at)}
                  </Typography>
                  <Typography variant="subtitle1" sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {cook.item_name}
                  </Typography>
                </Stack>
                <Typography variant="caption" color="text.secondary">
                  Not for a planned meal
                </Typography>
              </Stack>
              <Chip
                size="small"
                color="success"
                label={`Made ${cook.servings_produced}`}
                sx={{ flexShrink: 0 }}
              />
            </Stack>
          </Paper>
        ))}
      </Stack>
    </SlotSection>
  );
}
