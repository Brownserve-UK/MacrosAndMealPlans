import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Snackbar from '@mui/material/Snackbar';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useMember, useUpdateMember } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { ConflictDialog } from '../../components/ConflictDialog';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { roleLabel } from '../administration/roles';
import { NutritionTargetsPanel } from './NutritionTargetsPanel';

export function ProfilePage() {
  const { principal } = useAuth();
  if (!principal) return null;

  return (
    <>
      <PageHeader
        title={
          <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
            <InitialsAvatar name={principal.username} />
            <span>{principal.username}</span>
          </Stack>
        }
        subtitle="Your account and how you appear in the household."
      />

      <Stack spacing={3}>
        <Paper sx={{ p: 3 }}>
          <Typography variant="h3" sx={{ mb: 2 }}>
            Roles
          </Typography>
          <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1 }}>
            {principal.roles.map((role) => (
              <Chip key={role} size="small" label={roleLabel(role)} />
            ))}
          </Stack>
        </Paper>

        {principal.member_id ? (
          <>
            <NamePanel memberId={principal.member_id} />
            <NutritionTargetsPanel memberId={principal.member_id} />
          </>
        ) : (
          <Paper sx={{ p: 3 }}>
            <Typography variant="h3" sx={{ mb: 1 }}>
              Name
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Your account isn't linked to anyone in the household yet.
            </Typography>
          </Paper>
        )}
      </Stack>
    </>
  );
}

function NamePanel({ memberId }: { memberId: string }) {
  const query = useMember(memberId);

  if (query.isLoading) return <Loading label="Loading" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  if (!query.data) return null;

  return <EditName memberId={memberId} name={query.data.display_name} revision={query.data.revision} onReload={() => void query.refetch()} />;
}

function EditName({
  memberId,
  name: initialName,
  revision,
  onReload,
}: {
  memberId: string;
  name: string;
  revision: number;
  onReload: () => void;
}) {
  const update = useUpdateMember();
  const [name, setName] = useState(initialName);
  const [error, setError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);

  const dirty = name.trim() !== initialName;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    if (!name.trim()) {
      setError('Give yourself a name');
      return;
    }
    try {
      await update.mutateAsync({
        id: memberId,
        revision,
        body: { display_name: name.trim() },
      });
      setSaved(true);
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else setError(Object.values(caught.fieldErrors)[0] ?? caught.message);
      }
    }
  }

  return (
    <Paper sx={{ p: 3 }}>
      <Typography variant="h3" sx={{ mb: 2 }}>
        Name
      </Typography>
      <form onSubmit={onSubmit}>
        <Stack spacing={2} sx={{ alignItems: 'flex-start' }}>
          <TextField
            label="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            error={Boolean(error)}
            helperText={error}
            fullWidth
          />
          <Button type="submit" variant="contained" disabled={!dirty || update.isPending}>
            {update.isPending ? 'Saving…' : 'Save'}
          </Button>
        </Stack>
      </form>

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
    </Paper>
  );
}
