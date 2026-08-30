import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import FormControlLabel from '@mui/material/FormControlLabel';
import Paper from '@mui/material/Paper';
import Snackbar from '@mui/material/Snackbar';
import Stack from '@mui/material/Stack';
import Switch from '@mui/material/Switch';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type MealTimesSettings } from '../../api/client';
import type { components } from '../../api/schema';
import { useMealTimes, useUpdateMealTimes } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';

const SLOTS = [
  { key: 'breakfast', label: 'Breakfast' },
  { key: 'lunch', label: 'Lunch' },
  { key: 'dinner', label: 'Dinner' },
] as const;

export function MealTimesPage() {
  const query = useMealTimes();

  if (query.isLoading) return <Loading label="Loading meal times" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  if (!query.data) return <ErrorState error={new Error('Not found')} />;

  return <EditMealTimes settings={query.data} onReload={() => void query.refetch()} />;
}

function EditMealTimes({
  settings,
  onReload,
}: {
  settings: MealTimesSettings;
  onReload: () => void;
}) {
  const { principal } = useAuth();
  const canManage = principal?.permissions.includes('household:write') ?? false;

  const update = useUpdateMealTimes();
  const [times, setTimes] = useState({
    breakfast: settings.breakfast,
    lunch: settings.lunch,
    dinner: settings.dinner,
  });
  const [failure, setFailure] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);
  const [defaultAll, setDefaultAll] = useState(settings.default_all_members_participate);

  const dirty =
    SLOTS.some(({ key }) => times[key] !== settings[key]) ||
    defaultAll !== settings.default_all_members_participate;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const body: components['schemas']['UpdateMealTimesRequest'] = {};
    for (const { key } of SLOTS) {
      if (times[key] !== settings[key]) body[key] = times[key];
    }
    if (defaultAll !== settings.default_all_members_participate) {
      body.default_all_members_participate = defaultAll;
    }
    try {
      await update.mutateAsync({ revision: settings.revision, body });
      setSaved(true);
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else setFailure(Object.values(caught.fieldErrors)[0] ?? caught.message);
      } else {
        setFailure('Could not save.');
      }
    }
  }

  return (
    <>
      <PageHeader
        back={
          <Link to="/administration" className="app-link">
            <BackLabel>Administration</BackLabel>
          </Link>
        }
        title="Meal times"
        subtitle="Default times for the household's planned meals. New planned meals start at these times and can be adjusted per meal."
      />

      <Paper sx={{ p: 3, maxWidth: 420 }}>
        <form onSubmit={onSubmit}>
          <Stack spacing={3}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            {SLOTS.map(({ key, label }) => (
              <TextField
                key={key}
                type="time"
                label={label}
                value={times[key]}
                onChange={(event) => setTimes({ ...times, [key]: event.target.value })}
                disabled={!canManage}
                slotProps={{ inputLabel: { shrink: true } }}
                fullWidth
              />
            ))}

            <Typography variant="body2" color="text.secondary">
              Snacks have no set time.
            </Typography>

            <FormControlLabel
              control={
                <Switch
                  checked={defaultAll}
                  onChange={(event) => setDefaultAll(event.target.checked)}
                  disabled={!canManage}
                />
              }
              label="Add everyone to new household meals"
            />

            {canManage ? (
              <Button
                type="submit"
                variant="contained"
                disabled={!dirty || update.isPending}
                sx={{ alignSelf: 'flex-start' }}
              >
                {update.isPending ? 'Saving…' : 'Save'}
              </Button>
            ) : null}
          </Stack>
        </form>
      </Paper>

      <ConflictDialog
        error={conflict}
        onReload={() => {
          setConflict(null);
          onReload();
        }}
        onDismiss={() => setConflict(null)}
      />
      <Snackbar
        open={saved}
        autoHideDuration={2500}
        onClose={() => setSaved(false)}
        message="Saved"
      />
    </>
  );
}
