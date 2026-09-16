import type { SlotAttendance } from '../../api/client';

export function memberBlockedReason(
  rows: SlotAttendance[],
  memberId: string,
  includedMemberIds: string[] = [],
): string | null {
  if (includedMemberIds.includes(memberId)) return null;
  const row = rows.find((candidate) => candidate.member_id === memberId);
  if (!row) return null;
  if (row.attendance === 'self_catering') return 'Has their own plan';
  if (row.attendance === 'opted_out') return 'Opted out';
  if (row.attendance === 'participating') {
    return row.claimed_time ? `Already eating at ${row.claimed_time}` : 'Already in another meal';
  }
  return null;
}
