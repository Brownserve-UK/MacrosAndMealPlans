import MoreHorizIcon from '@mui/icons-material/MoreHorizOutlined';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import ListItemText from '@mui/material/ListItemText';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import { useState } from 'react';

export function RecordMenu({
  archived,
  onToggleArchive,
  origin,
  locallyEdited,
  updatedAt,
}: {
  archived: boolean;
  onToggleArchive: () => void;
  origin?: string;
  locallyEdited?: boolean;
  updatedAt: string;
}) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  const provenance =
    origin === undefined
      ? null
      : origin === 'seeded'
        ? locallyEdited
          ? 'Built in, edited'
          : 'Built in'
        : origin === 'external'
          ? 'Imported'
          : 'Added here';

  return (
    <>
      <IconButton onClick={(e) => setAnchor(e.currentTarget)} aria-label="More options">
        <MoreHorizIcon />
      </IconButton>
      <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
        <MenuItem
          onClick={() => {
            setAnchor(null);
            onToggleArchive();
          }}
        >
          <ListItemText>{archived ? 'Restore' : 'Archive'}</ListItemText>
        </MenuItem>
        <Divider />
        <Typography variant="caption" color="text.secondary" sx={{ px: 2, py: 1, display: 'block' }}>
          {provenance ? (
            <>
              {provenance}
              <br />
            </>
          ) : null}
          Last changed {new Date(updatedAt).toLocaleDateString('en-GB')}
        </Typography>
      </Menu>
    </>
  );
}
