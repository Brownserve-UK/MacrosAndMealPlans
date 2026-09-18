import Box from '@mui/material/Box';
import InputBase from '@mui/material/InputBase';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { IconTile } from '../../../components/IconTile';
import { KindChip } from '../../../components/KindChip';
import { groupCaption, groupDiners, groupKind } from './plannerWeek';
import type { GroupView, OccasionView, PlannerMember } from './types';

export function CookingRow({
  occasion,
  group,
  members,
  busy,
  onCooking,
}: {
  occasion: OccasionView;
  group: GroupView;
  members: PlannerMember[];
  busy: boolean;
  onCooking: (value: number | null) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState('');

  const kind = groupKind(group);
  const diners = groupDiners(occasion, group, members);
  const variations = group.participants.filter((participant) => participant.note != null).length;
  const captionParts = [groupCaption(group)];
  if (variations > 0) captionParts.push(`${variations} ${variations === 1 ? 'variation' : 'variations'}`);
  const caption = captionParts.filter((part): part is string => Boolean(part)).join(' · ');
  const extra = group.effective_cooking_servings - group.serves;

  function commit() {
    const value = Number(text);
    setEditing(false);
    if (text.trim() === '' || Number.isNaN(value) || value === group.serves) onCooking(null);
    else onCooking(Math.max(0, Math.round(value)));
  }

  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateColumns: '34px 1fr auto',
        gap: 1.5,
        alignItems: 'center',
        py: 1.5,
        borderTop: '1px solid',
        borderColor: 'divider',
      }}
    >
      <IconTile
        concept={kind === 'recipe' ? 'recipe' : kind === 'dish' ? 'dish' : kind === 'product' ? 'food' : 'meal'}
        tone={kind === 'recipe' || kind === 'saved' ? 'primary' : kind === 'dish' ? 'secondary' : 'neutral'}
      />
      <Box sx={{ minWidth: 0 }}>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
          <Typography sx={{ fontWeight: 500 }}>{group.name}</Typography>
          {kind === 'recipe' ? <KindChip kind="recipe" /> : null}
          {kind === 'product' ? <KindChip kind="product" /> : null}
          {kind === 'saved' ? <KindChip kind="saved_meal" /> : null}
        </Stack>
        {caption ? (
          <Typography variant="caption" color="text.secondary" className="numeral">
            {caption}
          </Typography>
        ) : null}
      </Box>
      {group.ad_hoc ? (
        <Typography variant="body2" color="text.secondary" className="numeral" sx={{ whiteSpace: 'nowrap' }}>
          {diners.length + group.guest_count} {diners.length + group.guest_count === 1 ? 'person' : 'people'}
        </Typography>
      ) : editing ? (
        <InputBase
          autoFocus
          value={text}
          inputProps={{
            'aria-label': `How much to cook for ${group.name}`,
            inputMode: 'numeric',
            style: { width: 48, textAlign: 'right' },
          }}
          onChange={(event) => setText(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === 'Enter') commit();
            if (event.key === 'Escape') setEditing(false);
          }}
          sx={{ border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1, fontSize: '0.875rem' }}
        />
      ) : (
        <Box
          component="button"
          type="button"
          disabled={busy}
          aria-label={`Change how much to cook for ${group.name}`}
          onClick={() => {
            setText(String(group.effective_cooking_servings));
            setEditing(true);
          }}
          sx={{ background: 'none', border: 0, p: 0, font: 'inherit', textAlign: 'right', cursor: 'pointer', color: 'text.secondary' }}
        >
          <Typography variant="body2" className="numeral" sx={{ whiteSpace: 'nowrap' }}>
            <Box component="b" sx={{ color: 'text.primary', fontWeight: 600 }}>
              {group.effective_cooking_servings}
            </Box>{' '}
            {group.effective_cooking_servings === 1 ? 'serving' : 'servings'}
          </Typography>
          {extra > 0 ? (
            <Typography variant="caption" color="text.secondary" className="numeral" sx={{ display: 'block' }}>
              {`${group.serves} eating, +${extra} spare`}
            </Typography>
          ) : null}
        </Box>
      )}
    </Box>
  );
}
