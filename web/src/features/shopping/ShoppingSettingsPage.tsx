import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import type { ShoppingCadence } from '../../api/client';
import {
  useClearShoppingCadence,
  useSetShoppingCadence,
  useShoppingCadence,
} from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { startOfWeekIso, todayIso } from '../meal-plan/date';

const DAYS = [
  { value: 1, label: 'Mon' },
  { value: 2, label: 'Tue' },
  { value: 3, label: 'Wed' },
  { value: 4, label: 'Thu' },
  { value: 5, label: 'Fri' },
  { value: 6, label: 'Sat' },
  { value: 7, label: 'Sun' },
];

export function ShoppingSettingsPage() {
  const cadence = useShoppingCadence();

  if (cadence.isLoading) return <Loading label="Loading your shopping schedule" />;
  if (cadence.isError) {
    return <ErrorState error={cadence.error} onRetry={() => cadence.refetch()} />;
  }

  return (
    <EditCadence
      key={cadence.data ? 'configured' : 'none'}
      cadence={cadence.data ?? null}
    />
  );
}

function EditCadence({ cadence }: { cadence: ShoppingCadence | null }) {
  const save = useSetShoppingCadence();
  const clear = useClearShoppingCadence();

  const [interval, setInterval] = useState(cadence?.interval_weeks ?? 1);
  const [days, setDays] = useState<number[]>(cadence?.days ?? [6]);
  const [failure, setFailure] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    setSaved(false);
    if (days.length === 0) {
      setFailure('Choose at least one day.');
      return;
    }
    try {
      await save.mutateAsync({
        interval_weeks: interval,
        days: [...days].sort((a, b) => a - b),
        anchor: startOfWeekIso(todayIso()),
      });
      setSaved(true);
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not save.');
    }
  }

  return (
    <>
      <PageHeader title="Shopping" subtitle="When you normally shop." />

      <Paper variant="outlined" sx={{ p: 3, maxWidth: 560 }}>
        <form onSubmit={onSubmit}>
          <Stack spacing={3}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            {saved ? <Alert severity="success">Saved.</Alert> : null}
            {!cadence && (
              <Typography variant="body2" color="text.secondary">
                Until you set this, we'll still work out what you need, but we can't say which
                trip to buy it on.
              </Typography>
            )}

            <TextField
              select
              label="How often"
              value={interval}
              onChange={(e) => setInterval(Number(e.target.value))}
              sx={{ maxWidth: 220 }}
            >
              <MenuItem value={1}>Every week</MenuItem>
              <MenuItem value={2}>Every 2 weeks</MenuItem>
              <MenuItem value={3}>Every 3 weeks</MenuItem>
              <MenuItem value={4}>Every 4 weeks</MenuItem>
            </TextField>

            <Stack spacing={1}>
              <Typography variant="body2" color="text.secondary">
                Which days
              </Typography>
              <ToggleButtonGroup
                value={days}
                onChange={(_, next: number[]) => setDays(next)}
                size="small"
                sx={{ flexWrap: 'wrap' }}
              >
                {DAYS.map((day) => (
                  <ToggleButton key={day.value} value={day.value} aria-label={day.label}>
                    {day.label}
                  </ToggleButton>
                ))}
              </ToggleButtonGroup>
            </Stack>

            <Stack direction="row" spacing={1.5}>
              <Button type="submit" variant="contained" disabled={save.isPending}>
                {save.isPending ? 'Saving…' : 'Save'}
              </Button>
              {cadence && (
                <Button
                  color="inherit"
                  disabled={clear.isPending}
                  onClick={() => clear.mutate()}
                >
                  Clear
                </Button>
              )}
            </Stack>
          </Stack>
        </form>
      </Paper>
    </>
  );
}
