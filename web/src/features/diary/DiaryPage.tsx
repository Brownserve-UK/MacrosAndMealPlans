import AddIcon from '@mui/icons-material/AddOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import RestaurantIcon from '@mui/icons-material/RestaurantOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import type { DiaryEntry } from '../../api/client';
import { useDiaryDay, useDiaryMembers } from '../../api/queries';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { MaybeNumber } from '../../components/Unknown';
import { DailyNutritionSummary } from './DailyNutritionSummary';
import { DayNav } from './DayNav';
import { EditEntryDialog } from './EditEntryDialog';
import { MemberSelect } from './MemberSelect';
import { NewEntryDialog } from './NewEntryDialog';
import { formatAmount, formatTime } from './format';

function EntryRow({
  entry,
  divided,
  onClick,
}: {
  entry: DiaryEntry;
  divided: boolean;
  onClick: () => void;
}) {
  return (
    <ButtonBase
      onClick={onClick}
      aria-label={`Edit ${entry.product_name}`}
      sx={{
        display: 'flex',
        width: '100%',
        alignItems: 'center',
        gap: { xs: 1.5, sm: 2 },
        px: { xs: 2, sm: 2.5 },
        py: 1.75,
        textAlign: 'left',
        borderTop: divided ? '1px solid' : 'none',
        borderColor: 'divider',
        transition: 'background-color 120ms ease',
        '&:hover': { backgroundColor: 'action.hover' },
        '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main', outlineOffset: -2 },
      }}
    >
      <Typography
        className="numeral"
        variant="caption"
        color="text.secondary"
        sx={{ width: 38, flexShrink: 0, display: { xs: 'none', sm: 'block' } }}
      >
        {formatTime(entry.consumed_at)}
      </Typography>
      <InitialsAvatar name={entry.product_name} size={44} />
      <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
        <Typography
          variant="subtitle1"
          sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
        >
          {entry.product_name}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          <Box component="span" sx={{ display: { sm: 'none' } }}>
            {formatTime(entry.consumed_at)} ·{' '}
          </Box>
          {formatAmount(entry.amount)}
        </Typography>
      </Stack>
      {entry.quality === 'unknown' ? (
        <Chip size="small" variant="outlined" label="No nutrition" sx={{ flexShrink: 0 }} />
      ) : (
        <Typography className="numeral" variant="body2" sx={{ flexShrink: 0, fontWeight: 600 }}>
          <MaybeNumber value={entry.nutrition.energy_kcal} fractionDigits={0} />{' '}
          <Box component="span" sx={{ color: 'text.secondary', fontSize: '0.75rem' }}>
            kcal
          </Box>
        </Typography>
      )}
      <ChevronRightIcon sx={{ color: 'text.disabled', fontSize: 20, flexShrink: 0 }} />
    </ButtonBase>
  );
}

function EmptyDiary({ onAdd }: { onAdd: () => void }) {
  return (
    <Paper sx={{ px: 3, py: { xs: 5, sm: 6 }, textAlign: 'center' }}>
      <Box
        sx={{
          width: 52,
          height: 52,
          mx: 'auto',
          mb: 2,
          display: 'grid',
          placeItems: 'center',
          borderRadius: '50%',
          color: 'primary.main',
          backgroundColor: 'action.selected',
        }}
      >
        <RestaurantIcon />
      </Box>
      <Typography variant="h3" sx={{ mb: 0.75 }}>
        No food logged
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
        Add the first item for this day.
      </Typography>
      <Button variant="contained" startIcon={<AddIcon />} onClick={onAdd}>
        Log food
      </Button>
    </Paper>
  );
}

export function DiaryPage({ memberId, date }: { memberId: string; date: string }) {
  const navigate = useNavigate();
  const membersQuery = useDiaryMembers();
  const dayQuery = useDiaryDay(memberId, date);
  const [addOpen, setAddOpen] = useState(false);
  const [editing, setEditing] = useState<DiaryEntry | null>(null);

  function goTo(nextMemberId: string, nextDate: string) {
    void navigate({
      to: '/diary/$memberId/$date',
      params: { memberId: nextMemberId, date: nextDate },
    });
  }

  if (dayQuery.isError) {
    return <ErrorState error={dayQuery.error} onRetry={() => dayQuery.refetch()} />;
  }

  const members = membersQuery.data ?? [];
  const entries = dayQuery.data?.entries ?? [];
  const incompleteCount = dayQuery.data
    ? dayQuery.data.totals.unknown_count + dayQuery.data.totals.partial_count
    : 0;

  return (
    <Box sx={{ maxWidth: 980, mx: 'auto' }}>
      <PageHeader
        title="Food diary"
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            Log food
          </Button>
        }
      />

      <Paper
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', sm: '1fr auto 1fr' },
          alignItems: 'center',
          gap: 2,
          px: { xs: 1.5, sm: 2 },
          py: 1.5,
          mb: 3,
          backgroundColor: 'background.default',
        }}
      >
        <Box sx={{ display: { xs: 'none', sm: 'block' } }} />
        <DayNav date={date} onChange={(next) => goTo(memberId, next)} />
        {members.length > 1 ? (
          <Box sx={{ justifySelf: { sm: 'end' }, width: { xs: '100%', sm: 'auto' } }}>
            <MemberSelect
              members={members}
              value={memberId}
              onChange={(next) => goTo(next, date)}
            />
          </Box>
        ) : null}
      </Paper>

      {dayQuery.isLoading ? (
        <Loading label="Loading diary" />
      ) : (
        <Stack spacing={3.5}>
          {entries.length > 0 ? (
            <Box component="section" aria-labelledby="nutrition-heading">
              <Typography id="nutrition-heading" variant="h3" sx={{ mb: 1.25 }}>
                Nutrition
              </Typography>
              <DailyNutritionSummary
                nutrition={dayQuery.data?.totals.nutrition}
                incompleteCount={incompleteCount}
              />
            </Box>
          ) : null}

          <Box component="section" aria-labelledby="food-log-heading">
            <Stack
              direction="row"
              sx={{ mb: 1.25, alignItems: 'baseline', justifyContent: 'space-between' }}
            >
              <Typography id="food-log-heading" variant="h2">
                Food logged
              </Typography>
              {entries.length > 0 ? (
                <Typography variant="caption" color="text.secondary">
                  {entries.length === 1 ? '1 item' : `${entries.length} items`}
                </Typography>
              ) : null}
            </Stack>

            {entries.length === 0 ? (
              <EmptyDiary onAdd={() => setAddOpen(true)} />
            ) : (
              <Paper sx={{ overflow: 'hidden' }}>
                {entries.map((entry, index) => (
                  <EntryRow
                    key={entry.id}
                    entry={entry}
                    divided={index > 0}
                    onClick={() => setEditing(entry)}
                  />
                ))}
                <Button
                  fullWidth
                  startIcon={<AddIcon />}
                  onClick={() => setAddOpen(true)}
                  sx={{ py: 1.5, borderTop: '1px solid', borderColor: 'divider', borderRadius: 0 }}
                >
                  Add another item
                </Button>
              </Paper>
            )}
          </Box>
        </Stack>
      )}

      <NewEntryDialog
        open={addOpen}
        onClose={() => setAddOpen(false)}
        memberId={memberId}
        date={date}
      />
      {editing ? (
        <EditEntryDialog open record={editing} onClose={() => setEditing(null)} />
      ) : null}
    </Box>
  );
}
