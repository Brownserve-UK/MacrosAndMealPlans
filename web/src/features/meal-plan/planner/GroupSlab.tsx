import Box from '@mui/material/Box';
import InputBase from '@mui/material/InputBase';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { IconTile } from '../../../components/IconTile';
import { KindChip } from '../../../components/KindChip';
import { EveryonePill, GhostPill, PersonPill } from './PersonPill';
import { groupCaption, groupDiners, groupKind } from './plannerWeek';
import type { GroupView, OccasionView, PlannerMember } from './types';

type PillMenu = { anchor: HTMLElement; member: PlannerMember };

export function GroupSlab({
  occasion,
  group,
  members,
  showOff,
  busy,
  onUntick,
  onRetick,
  onVariation,
  onGuests,
  onCooking,
}: {
  occasion: OccasionView;
  group: GroupView;
  members: PlannerMember[];
  showOff: PlannerMember[];
  busy: boolean;
  onUntick: (member: PlannerMember) => void;
  onRetick: (member: PlannerMember) => void;
  onVariation: (member: PlannerMember, note: string | null) => void;
  onGuests: (count: number) => void;
  onCooking: (value: number | null) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [pillMenu, setPillMenu] = useState<PillMenu | null>(null);
  const [guestMenu, setGuestMenu] = useState<HTMLElement | null>(null);
  const [variationFor, setVariationFor] = useState<PlannerMember | null>(null);
  const [variationText, setVariationText] = useState('');
  const [editingCooking, setEditingCooking] = useState(false);
  const [cookingText, setCookingText] = useState('');

  const kind = groupKind(group);
  const diners = groupDiners(occasion, group, members);
  const caption = groupCaption(group);
  const showEveryone = group.everyone && !expanded && diners.length === members.length && members.length > 1;
  const extra = group.effective_cooking_servings - group.serves;

  function noteFor(member: PlannerMember) {
    return group.participants.find((participant) => participant.member_id === member.id)?.note ?? null;
  }

  function commitVariation() {
    if (!variationFor) return;
    const trimmed = variationText.trim();
    onVariation(variationFor, trimmed === '' ? null : trimmed);
    setVariationFor(null);
  }

  function commitCooking() {
    const value = Number(cookingText);
    setEditingCooking(false);
    if (cookingText.trim() === '' || Number.isNaN(value) || value === group.serves) onCooking(null);
    else onCooking(Math.max(0, Math.round(value)));
  }

  return (
    <Box sx={{ p: 2, borderRadius: '10px', backgroundColor: 'background.default' }}>
      <Stack spacing={1.75}>
        <Box sx={{ display: 'grid', gridTemplateColumns: '34px 1fr auto', gap: 1.5, alignItems: 'center' }}>
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
          ) : editingCooking ? (
            <InputBase
              autoFocus
              value={cookingText}
              inputProps={{ 'aria-label': 'Cooking servings', inputMode: 'numeric', style: { width: 48, textAlign: 'right' } }}
              onChange={(event) => setCookingText(event.target.value)}
              onBlur={commitCooking}
              onKeyDown={(event) => {
                if (event.key === 'Enter') commitCooking();
                if (event.key === 'Escape') setEditingCooking(false);
              }}
              sx={{ border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1, fontSize: '0.875rem' }}
            />
          ) : (
            <Box
              component="button"
              type="button"
              disabled={busy}
              aria-label="Change how much is being cooked"
              onClick={() => {
                setCookingText(String(group.effective_cooking_servings));
                setEditingCooking(true);
              }}
              sx={{ background: 'none', border: 0, p: 0, font: 'inherit', textAlign: 'right', cursor: 'pointer', color: 'text.secondary' }}
            >
              <Typography variant="body2" className="numeral" sx={{ whiteSpace: 'nowrap' }}>
                Serves <Box component="b" sx={{ color: 'text.primary' }}>{group.serves}</Box>
              </Typography>
              {extra !== 0 ? (
                <Typography variant="body2" className="numeral" sx={{ whiteSpace: 'nowrap' }}>
                  Cooking <Box component="b" sx={{ color: 'text.primary' }}>{group.effective_cooking_servings}</Box>
                  {extra > 0 ? ` (+${extra} for leftovers)` : ''}
                </Typography>
              ) : null}
            </Box>
          )}
        </Box>

        <Stack direction="row" sx={{ flexWrap: 'wrap', gap: 1, pl: '46px' }}>
          {showEveryone ? (
            <EveryonePill members={diners} onClick={() => setExpanded(true)} />
          ) : (
            diners.map((member) => (
              <PersonPill
                key={member.id}
                member={member}
                variation={noteFor(member)}
                onClick={(event) => setPillMenu({ anchor: event.currentTarget, member })}
              />
            ))
          )}
          {showOff.map((member) => (
            <PersonPill key={member.id} member={member} off onClick={() => onRetick(member)} />
          ))}
          {group.guest_count > 0 ? (
            <GhostPill
              label={group.guest_count === 1 ? '1 guest' : `${group.guest_count} guests`}
              onClick={(event) => setGuestMenu(event.currentTarget)}
            />
          ) : (
            <GhostPill label="Guest" onClick={() => onGuests(1)} />
          )}
        </Stack>

        {variationFor ? (
          <Stack direction="row" spacing={1} sx={{ pl: '46px', alignItems: 'center' }}>
            <InputBase
              autoFocus
              value={variationText}
              placeholder={`How ${variationFor.name} has it`}
              inputProps={{ 'aria-label': `Variation for ${variationFor.name}` }}
              onChange={(event) => setVariationText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') commitVariation();
                if (event.key === 'Escape') setVariationFor(null);
              }}
              onBlur={commitVariation}
              sx={{ border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1.25, py: 0.25, fontSize: '0.875rem', flexGrow: 1 }}
            />
          </Stack>
        ) : null}
      </Stack>

      <Menu open={Boolean(pillMenu)} anchorEl={pillMenu?.anchor ?? null} onClose={() => setPillMenu(null)}>
        <MenuItem
          onClick={() => {
            const member = pillMenu?.member ?? null;
            setPillMenu(null);
            if (!member) return;
            setVariationText(noteFor(member) ?? '');
            setVariationFor(member);
          }}
        >
          {pillMenu && noteFor(pillMenu.member) ? 'Change the variation' : 'Add a variation'}
        </MenuItem>
        <MenuItem
          onClick={() => {
            const member = pillMenu?.member ?? null;
            setPillMenu(null);
            if (member) onUntick(member);
          }}
        >
          Not eating this
        </MenuItem>
      </Menu>

      <Menu open={Boolean(guestMenu)} anchorEl={guestMenu} onClose={() => setGuestMenu(null)}>
        <MenuItem
          onClick={() => {
            setGuestMenu(null);
            onGuests(group.guest_count + 1);
          }}
        >
          Add a guest
        </MenuItem>
        <MenuItem
          onClick={() => {
            setGuestMenu(null);
            onGuests(Math.max(0, group.guest_count - 1));
          }}
        >
          Remove a guest
        </MenuItem>
      </Menu>
    </Box>
  );
}
