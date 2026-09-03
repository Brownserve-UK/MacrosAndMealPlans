import AddIcon from '@mui/icons-material/AddOutlined';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type Role, type User } from '../../api/client';
import { useCreateUser, useSetUserArchived, useSetUserRoles, useUsers } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { BackLabel } from '../../components/BackLink';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { ROLES, describeRoles } from './roles';

function RolePicker({
  value,
  onChange,
}: {
  value: Role[];
  onChange: (next: Role[]) => void;
}) {
  return (
    <Stack>
      {ROLES.map((role) => (
        <FormControlLabel
          key={role.value}
          control={
            <Checkbox
              checked={value.includes(role.value)}
              onChange={(e) =>
                onChange(
                  e.target.checked
                    ? [...value, role.value]
                    : value.filter((r) => r !== role.value),
                )
              }
            />
          }
          label={
            <Stack>
              <Typography variant="body2">{role.label}</Typography>
              <Typography variant="caption" color="text.secondary">
                {role.hint}
              </Typography>
            </Stack>
          }
        />
      ))}
    </Stack>
  );
}

function NewAccountDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const create = useCreateUser();
  const [username, setUsername] = useState('');
  const [roles, setRoles] = useState<Role[]>(['basic_user']);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  function handleClose() {
    setUsername('');
    setRoles(['basic_user']);
    setErrors({});
    setFailure(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    try {
      await create.mutateAsync({ username: username.trim(), roles });
      handleClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        const fields = caught.fieldErrors;
        if (Object.keys(fields).length > 0) setErrors(fields);
        else setFailure(caught.message);
      } else setFailure('Could not save.');
    }
  }

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>New account</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <TextField
              label="Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              error={Boolean(errors.username)}
              helperText={errors.username}
              autoFocus
              fullWidth
              required
            />
            <RolePicker value={roles} onChange={setRoles} />
            {errors.roles ? <Alert severity="error">{errors.roles}</Alert> : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Create'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}

function EditRolesDialog({ user, onClose }: { user: User | null; onClose: () => void }) {
  const setRoles = useSetUserRoles();
  const setArchived = useSetUserArchived();
  const [roles, setLocalRoles] = useState<Role[]>(user?.roles ?? []);
  const [failure, setFailure] = useState<string | null>(null);

  if (!user) return null;

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!user) return;
    setFailure(null);
    try {
      await setRoles.mutateAsync({ id: user.id, revision: user.revision, roles });
      onClose();
    } catch (caught) {
      setFailure(
        caught instanceof ApiError
          ? (Object.values(caught.fieldErrors)[0] ?? caught.message)
          : 'Could not save.',
      );
    }
  }

  async function toggleArchive() {
    if (!user) return;
    setFailure(null);
    try {
      await setArchived.mutateAsync({
        id: user.id,
        revision: user.revision,
        archived: !user.archived_at,
      });
      onClose();
    } catch (caught) {
      setFailure(
        caught instanceof ApiError
          ? (Object.values(caught.fieldErrors)[0] ?? caught.message)
          : 'Could not save.',
      );
    }
  }

  return (
    <FormDialog open onClose={onClose} maxWidth="sm" fullWidth>
      <form onSubmit={save}>
        <DialogTitle>{user.username}</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <RolePicker value={roles} onChange={setLocalRoles} />
          </Stack>
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'space-between' }}>
          <Button color="inherit" onClick={() => void toggleArchive()}>
            {user.archived_at ? 'Restore' : 'Archive'}
          </Button>
          <Stack direction="row" spacing={1}>
            <Button onClick={onClose}>Cancel</Button>
            <Button type="submit" variant="contained" disabled={setRoles.isPending}>
              {setRoles.isPending ? 'Saving…' : 'Save'}
            </Button>
          </Stack>
        </DialogActions>
      </form>
    </FormDialog>
  );
}

export function AccountsPage() {
  const { principal } = useAuth();
  const [addOpen, setAddOpen] = useState(false);
  const [editing, setEditing] = useState<User | null>(null);

  const canManage = principal?.permissions.includes('account:admin') ?? false;
  const query = useUsers({ include_archived: true, per_page: 200 });

  if (!canManage) {
    return (
      <EmptyState
        title="Not available"
        description="Managing accounts needs admin access."
      />
    );
  }

  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const items = query.data?.items ?? [];

  return (
    <>
      <PageHeader
        back={
          <Link to="/administration" className="app-link">
            <BackLabel>Administration</BackLabel>
          </Link>
        }
        title="Accounts"
        subtitle="Who can sign in, and what they are allowed to do."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            New account
          </Button>
        }
      />

      {query.isLoading ? (
        <Loading label="Loading accounts" />
      ) : items.length === 0 ? (
        <EmptyState
          title="No accounts yet"
          description="Create an account for anyone who needs to sign in."
          action={
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
              New account
            </Button>
          }
        />
      ) : (
        <RecordListShell>
          {items.map((user) => (
            <a
              key={user.id}
              href="#edit"
              onClick={(e) => {
                e.preventDefault();
                setEditing(user);
              }}
            >
              <RecordRow
                name={user.username}
                detail={describeRoles(user.roles)}
                muted={Boolean(user.archived_at)}
                trailing={
                  user.archived_at ? (
                    <Chip size="small" variant="outlined" label="Archived" />
                  ) : null
                }
              />
            </a>
          ))}
        </RecordListShell>
      )}

      <NewAccountDialog open={addOpen} onClose={() => setAddOpen(false)} />
      <EditRolesDialog
        key={editing?.id ?? 'none'}
        user={editing}
        onClose={() => setEditing(null)}
      />
    </>
  );
}
