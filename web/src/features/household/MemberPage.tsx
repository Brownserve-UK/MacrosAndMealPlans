import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Grid from '@mui/material/Grid';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Snackbar from '@mui/material/Snackbar';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type Member } from '../../api/client';
import {
  useMember,
  useSetMemberAccount,
  useSetMemberArchived,
  useUpdateMember,
  useUsers,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { RecordMenu } from '../../components/RecordMenu';
import { ErrorState, Loading } from '../../components/States';

export function MemberPage({ id }: { id: string }) {
  const query = useMember(id);

  if (query.isLoading) return <Loading label="Loading" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  if (!query.data) return <ErrorState error={new Error('Not found')} />;

  return <EditMember member={query.data} onReload={() => void query.refetch()} />;
}

function AccountPanel({ member }: { member: Member }) {
  const { principal } = useAuth();
  const canManageAccounts = principal?.permissions.includes('account:admin') ?? false;
  const setAccount = useSetMemberAccount();
  const [choice, setChoice] = useState('');
  const [failure, setFailure] = useState<string | null>(null);

  const users = useUsers({ per_page: 200 });
  const taken = new Set<string>();
  const available = (users.data?.items ?? []).filter((u) => !taken.has(u.id));

  async function change(userId: string | null) {
    setFailure(null);
    try {
      await setAccount.mutateAsync({ id: member.id, revision: member.revision, userId });
      setChoice('');
    } catch (caught) {
      setFailure(
        caught instanceof ApiError
          ? (Object.values(caught.fieldErrors)[0] ?? caught.message)
          : 'Could not save.',
      );
    }
  }

  return (
    <Paper sx={{ p: 3 }}>
      <Typography variant="h3" sx={{ mb: 2.5 }}>
        Account
      </Typography>

      <Stack spacing={2}>
        {failure ? <Alert severity="error">{failure}</Alert> : null}

        {member.linked_user_id ? (
          <>
            <Typography variant="body2" color="text.secondary">
              Signs in as{' '}
              <strong>
                {(users.data?.items ?? []).find((u) => u.id === member.linked_user_id)?.username ??
                  'an account'}
              </strong>
              .
            </Typography>
            {canManageAccounts ? (
              <Button
                size="small"
                color="inherit"
                onClick={() => void change(null)}
                disabled={setAccount.isPending}
                sx={{ alignSelf: 'flex-start' }}
              >
                Unlink
              </Button>
            ) : null}
          </>
        ) : canManageAccounts ? (
          <>
            <Typography variant="body2" color="text.secondary">
              No account. They can still be planned for.
            </Typography>
            <Stack direction="row" spacing={1}>
              <TextField
                select
                size="small"
                label="Link an account"
                value={choice}
                onChange={(e) => setChoice(e.target.value)}
                sx={{ minWidth: 200 }}
              >
                {available.length === 0 ? (
                  <MenuItem value="" disabled>
                    No accounts yet
                  </MenuItem>
                ) : null}
                {available.map((user) => (
                  <MenuItem key={user.id} value={user.id}>
                    {user.username}
                  </MenuItem>
                ))}
              </TextField>
              <Button
                onClick={() => void change(choice)}
                disabled={!choice || setAccount.isPending}
              >
                Link
              </Button>
            </Stack>
          </>
        ) : (
          <Typography variant="body2" color="text.secondary">
            No account.
          </Typography>
        )}
      </Stack>
    </Paper>
  );
}

function EditMember({ member, onReload }: { member: Member; onReload: () => void }) {
  const { principal } = useAuth();
  const canManage = principal?.permissions.includes('household:write') ?? false;

  const update = useUpdateMember();
  const setArchived = useSetMemberArchived();

  const [name, setName] = useState(member.display_name);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);

  const dirty = name.trim() !== member.display_name;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setErrors({});
    if (!name.trim()) {
      setErrors({ display_name: 'Give them a name' });
      return;
    }
    try {
      await update.mutateAsync({
        id: member.id,
        revision: member.revision,
        body: { display_name: name.trim() },
      });
      setSaved(true);
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else setErrors(caught.fieldErrors);
      }
    }
  }

  return (
    <>
      <PageHeader
        back={
          <Link to="/household" className="app-link">
            <BackLabel>Household</BackLabel>
          </Link>
        }
        title={
          <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
            <InitialsAvatar name={member.display_name} />
            <span>{member.display_name}</span>
            {member.archived_at ? <Chip size="small" label="Archived" /> : null}
          </Stack>
        }
        actions={
          canManage ? (
            <RecordMenu
              archived={Boolean(member.archived_at)}
              onToggleArchive={() =>
                void setArchived.mutateAsync({
                  id: member.id,
                  revision: member.revision,
                  archived: !member.archived_at,
                })
              }
              updatedAt={member.updated_at}
            />
          ) : undefined
        }
      />

      <Grid container spacing={3}>
        <Grid size={{ xs: 12, md: 5 }}>
          <AccountPanel member={member} />
        </Grid>
        <Grid size={{ xs: 12, md: 7 }}>
          <Paper sx={{ p: 3 }}>
            <form onSubmit={onSubmit}>
              <Stack spacing={3}>
                <TextField
                  label="Name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  error={Boolean(errors.display_name)}
                  helperText={errors.display_name}
                  disabled={!canManage}
                  fullWidth
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
        </Grid>
      </Grid>

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
