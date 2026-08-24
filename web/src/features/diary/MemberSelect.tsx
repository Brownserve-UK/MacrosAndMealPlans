import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import type { Member } from '../../api/client';

export function MemberSelect({
  members,
  value,
  onChange,
}: {
  members: Member[];
  value: string;
  onChange: (next: string) => void;
}) {
  return (
    <TextField
      select
      label="Diary for"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      size="small"
      sx={{ minWidth: { sm: 180 }, width: { xs: '100%', sm: 'auto' } }}
    >
      {members.map((member) => (
        <MenuItem key={member.id} value={member.id}>
          {member.display_name}
        </MenuItem>
      ))}
    </TextField>
  );
}
