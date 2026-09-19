import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Link from '@mui/material/Link';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Snackbar from '@mui/material/Snackbar';
import Typography from '@mui/material/Typography';
import useMediaQuery from '@mui/material/useMediaQuery';
import { useTheme } from '@mui/material/styles';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { ApiError, type MealSlot } from '../../../api/client';
import {
  useAddGroup,
  useAddPlannerGuest,
  useCopyOccasion,
  useCopyWeek,
  useCreateOccasion,
  useDeleteOccasion,
  useMoveOccasion,
  usePlannerWeek,
  useSetAttendance,
  useUpdateOccasion,
} from '../../../api/queries';
import { ErrorState, Loading } from '../../../components/States';
import { useHouseholdTimeZone } from '../../../hooks/useHouseholdTimeZone';
import { addDays, parseIsoDate, startOfWeekIso, todayIso } from '../date';
import { AddMealPicker } from './AddMealPicker';
import { DayCards } from './DayCards';
import { DeleteOccasionDialog } from './DeleteOccasionDialog';
import { MoveCopyDialog } from './MoveCopyDialog';
import { OccasionCard } from './OccasionCard';
import { occasionTitle, toNewGroup } from './plannerWeek';
import type { NewGroup, OccasionView } from './types';
import { WeekGrid } from './WeekGrid';
import { WeekHeader } from './WeekHeader';

type Adding = { date: string; slot: MealSlot; initial: string; anchor: HTMLElement | null };
type CellMenu = { occasion: OccasionView; position: { left: number; top: number } };
type Moving = { mode: 'move' | 'copy'; occasion: OccasionView };
type Toast = { message: string; undo: (() => Promise<void>) | null };

export function PlannerPage({ weekStart }: { weekStart: string }) {
  const navigate = useNavigate();
  const theme = useTheme();
  const desktop = useMediaQuery(theme.breakpoints.up('md'));
  const timeZone = useHouseholdTimeZone();
  const today = todayIso(timeZone);
  const currentMonday = startOfWeekIso(today);
  const week = usePlannerWeek(weekStart);
  const create = useCreateOccasion();
  const remove = useDeleteOccasion();
  const move = useMoveOccasion();
  const copy = useCopyOccasion();
  const copyWeek = useCopyWeek();
  const addGroup = useAddGroup();
  const addGuest = useAddPlannerGuest();
  const updateOccasion = useUpdateOccasion();
  const setAttendance = useSetAttendance();

  const [adding, setAdding] = useState<Adding | null>(null);
  const [viewingId, setViewingId] = useState<string | null>(null);
  const [cellMenu, setCellMenu] = useState<CellMenu | null>(null);
  const [moving, setMoving] = useState<Moving | null>(null);
  const [deleting, setDeleting] = useState<OccasionView | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [error, setError] = useState<string | null>(null);

  const viewing = viewingId
    ? week.data?.days.flatMap((day) => day.occasions).find((occasion) => occasion?.id === viewingId) ?? null
    : null;
  const empty = week.data ? week.data.days.every((day) => day.occasions.every((occasion) => occasion === null)) : false;

  function goToWeek(start: string) {
    void navigate({ to: '/planner/$weekStart', params: { weekStart: start } });
  }

  async function run(action: () => Promise<unknown>, fallback: string) {
    try {
      setError(null);
      await action();
      return true;
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : fallback);
      return false;
    }
  }

  async function addMeal(group: NewGroup) {
    if (!adding) return;
    const target = adding;
    setAdding(null);
    await run(
      () => create.mutateAsync({ planned_on: target.date, slot: target.slot, group }),
      'Could not plan that meal.',
    );
  }

  async function restore(occasion: OccasionView) {
    const [first, ...rest] = occasion.groups;
    if (!first) return;
    const created = await create.mutateAsync({
      planned_on: occasion.planned_on,
      slot: occasion.slot,
      group: { ...toNewGroup(first), cooking_servings: first.cooking_servings },
    });
    let revision = created.revision;
    async function restoreGuests(group: (typeof occasion.groups)[number], groupId: string) {
      for (const guest of group.guests) {
        for (let index = 0; index < guest.count; index += 1) {
          const updated = await addGuest.mutateAsync({
            occasionId: created.id,
            revision,
            name: guest.name,
            note: guest.note,
            target: { group_id: groupId },
          });
          revision = updated.revision;
        }
      }
    }
    await restoreGuests(first, created.groups[0]!.id);
    for (const group of rest) {
      const added = await addGroup.mutateAsync({
        occasionId: created.id,
        body: { ...toNewGroup(group), cooking_servings: group.cooking_servings },
      });
      revision += 1;
      await restoreGuests(group, added.id);
    }
    for (const memberId of occasion.absent_member_ids) {
      const updated = await setAttendance.mutateAsync({ occasionId: created.id, memberId, attendance: { kind: 'elsewhere' } });
      revision = updated.revision;
    }
    if (occasion.planned_time || occasion.note) {
      await updateOccasion.mutateAsync({
        id: created.id,
        body: { planned_time: occasion.planned_time, note: occasion.note, revision },
      });
    }
  }

  async function deleteOccasion() {
    if (!deleting) return;
    const snapshot = deleting;
    const ok = await run(() => remove.mutateAsync({ id: snapshot.id, revision: snapshot.revision }), 'Could not delete that meal.');
    setDeleting(null);
    if (ok) {
      setViewingId(null);
      setToast({ message: `Deleted ${occasionTitle(snapshot)}`, undo: () => restore(snapshot) });
    }
  }

  async function moveOccasion(occasion: OccasionView, date: string, slot: MealSlot, mode: 'move' | 'copy') {
    setMoving(null);
    if (mode === 'copy') {
      await run(() => copy.mutateAsync({ id: occasion.id, planned_on: date, slot }), 'Could not copy that meal.');
      return;
    }
    const ok = await run(() => move.mutateAsync({ id: occasion.id, planned_on: date, slot }), 'Could not move that meal.');
    if (ok) {
      setViewingId(null);
      const when = parseIsoDate(date).toLocaleDateString('en-GB', { weekday: 'long' });
      setToast({
        message: `Moved to ${when}`,
        undo: async () => {
          await move.mutateAsync({ id: occasion.id, planned_on: occasion.planned_on, slot: occasion.slot });
        },
      });
    }
  }

  function onDrop(occasionId: string, date: string, slot: MealSlot, copyIt: boolean) {
    const occasion = week.data?.days.flatMap((day) => day.occasions).find((candidate) => candidate?.id === occasionId);
    if (!occasion) return;
    void moveOccasion(occasion, date, slot, copyIt ? 'copy' : 'move');
  }

  function startFromFavourites() {
    const date = week.data && today >= weekStart && today <= addDays(weekStart, 6) ? today : weekStart;
    setAdding({ date, slot: 'dinner', initial: '', anchor: null });
  }

  if (week.isError) return <ErrorState error={week.error} onRetry={() => week.refetch()} />;

  return (
    <Box>
      <WeekHeader weekStart={weekStart} currentMonday={currentMonday} onWeekChange={goToWeek} />
      {error ? (
        <Typography variant="body2" color="error" sx={{ mb: 2 }}>
          {error}
        </Typography>
      ) : null}
      {empty ? (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          Nothing planned yet.{' '}
          <Link
            component="button"
            type="button"
            underline="none"
            sx={{ fontWeight: 500 }}
            onClick={() =>
              void run(
                () => copyWeek.mutateAsync({ weekStart, sourceWeekStart: addDays(weekStart, -7) }),
                'Could not copy last week.',
              )
            }
          >
            Copy last week
          </Link>{' '}
          or{' '}
          <Link component="button" type="button" underline="none" sx={{ fontWeight: 500 }} onClick={startFromFavourites}>
            start from favourites
          </Link>
          .
        </Typography>
      ) : null}
      {week.isLoading || !week.data ? <Loading label="Loading planner" /> : null}
      {week.data ? (
        desktop ? (
          <WeekGrid
            week={week.data}
            weekStart={weekStart}
            today={today}
            onOpen={(occasion) => setViewingId(occasion.id)}
            onAdd={(date, slot, initial, anchor) => setAdding({ date, slot, initial, anchor })}
            onContextMenu={(occasion, position) => setCellMenu({ occasion, position })}
            onDrop={onDrop}
          />
        ) : (
          <DayCards
            week={week.data}
            weekStart={weekStart}
            today={today}
            onOpen={(occasion) => setViewingId(occasion.id)}
            onAdd={(date, slot, initial, anchor) => setAdding({ date, slot, initial, anchor })}
            onLongPress={(occasion, position) => setCellMenu({ occasion, position })}
          />
        )
      ) : null}

      <AddMealPicker
        open={adding !== null}
        anchorEl={adding?.anchor ?? null}
        sheet={!desktop || adding?.anchor === null}
        initialQuery={adding?.initial ?? ''}
        onPick={(group) => void addMeal(group)}
        onClose={() => setAdding(null)}
      />

      {viewing && week.data ? (
        <OccasionCard
          occasion={viewing}
          week={week.data}
          sheet={!desktop}
          onClose={() => setViewingId(null)}
          onMove={() => setMoving({ mode: 'move', occasion: viewing })}
          onCopy={() => setMoving({ mode: 'copy', occasion: viewing })}
          onDelete={() => setDeleting(viewing)}
        />
      ) : null}

      <Menu
        open={cellMenu !== null}
        onClose={() => setCellMenu(null)}
        anchorReference="anchorPosition"
        anchorPosition={cellMenu?.position}
      >
        <MenuItem
          onClick={() => {
            if (cellMenu) setMoving({ mode: 'move', occasion: cellMenu.occasion });
            setCellMenu(null);
          }}
        >
          Move
        </MenuItem>
        <MenuItem
          onClick={() => {
            if (cellMenu) setMoving({ mode: 'copy', occasion: cellMenu.occasion });
            setCellMenu(null);
          }}
        >
          Copy
        </MenuItem>
        <MenuItem
          sx={{ color: 'error.main' }}
          onClick={() => {
            if (cellMenu) setDeleting(cellMenu.occasion);
            setCellMenu(null);
          }}
        >
          Delete
        </MenuItem>
      </Menu>

      {moving && week.data ? (
        <MoveCopyDialog
          mode={moving.mode}
          occasion={moving.occasion}
          week={week.data}
          busy={move.isPending || copy.isPending}
          onConfirm={(date, slot) => void moveOccasion(moving.occasion, date, slot, moving.mode)}
          onClose={() => setMoving(null)}
        />
      ) : null}

      {deleting ? (
        <DeleteOccasionDialog
          title={occasionTitle(deleting)}
          busy={remove.isPending}
          onCancel={() => setDeleting(null)}
          onDelete={() => void deleteOccasion()}
        />
      ) : null}

      <Snackbar
        open={toast !== null}
        autoHideDuration={6000}
        onClose={(_, reason) => {
          if (reason !== 'clickaway') setToast(null);
        }}
        message={toast?.message}
        action={
          toast?.undo ? (
            <Button
              size="small"
              color="inherit"
              onClick={() => {
                const undo = toast.undo;
                setToast(null);
                if (undo) void run(undo, 'Could not undo that.');
              }}
            >
              Undo
            </Button>
          ) : null
        }
      />
    </Box>
  );
}
