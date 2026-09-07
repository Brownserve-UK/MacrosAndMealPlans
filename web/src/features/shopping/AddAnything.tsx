import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import InputAdornment from '@mui/material/InputAdornment';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { useAddShoppingListItem, useProducts, useRecordPurchase } from '../../api/queries';
import { sectionLabel } from './sections';

export function AddAnything({ date }: { date: string }) {
  const [typed, setTyped] = useState('');
  const add = useAddShoppingListItem();
  const record = useRecordPurchase();
  const search = typed.trim();
  const products = useProducts(search ? { q: search, per_page: 5 } : { per_page: 1 });
  const matches = search ? (products.data?.items ?? []) : [];

  function onList(name: string, productId?: string) {
    add.mutate(
      { name, product_id: productId, opportunity_date: date },
      { onSuccess: () => setTyped('') },
    );
  }

  function onBought(name: string, productId?: string) {
    record.mutate(
      { name, product_id: productId, opportunity_date: date },
      { onSuccess: () => setTyped('') },
    );
  }

  return (
    <Stack spacing={1}>
      <TextField
        value={typed}
        onChange={(e) => setTyped(e.target.value)}
        placeholder="Add anything"
        fullWidth
        slotProps={{
          input: {
            startAdornment: (
              <InputAdornment position="start">
                <AddIcon fontSize="small" />
              </InputAdornment>
            ),
          },
        }}
      />

      {search ? (
        <Paper variant="outlined" sx={{ overflow: 'hidden' }}>
          {matches.map((product) => (
            <Row
              key={product.id}
              title={product.name}
              caption={
                product.shopping_section ? sectionLabel(product.shopping_section) : undefined
              }
              onList={() => onList(product.name, product.id)}
              onBought={() => onBought(product.name, product.id)}
            />
          ))}
          <Row
            title={`Use “${search}” as it is`}
            caption="Sort out the details later"
            onList={() => onList(search)}
            onBought={() => onBought(search)}
          />
        </Paper>
      ) : null}
    </Stack>
  );
}

function Row({
  title,
  caption,
  onList,
  onBought,
}: {
  title: string;
  caption?: string;
  onList: () => void;
  onBought: () => void;
}) {
  return (
    <Stack
      direction="row"
      spacing={1}
      sx={{
        alignItems: 'center',
        px: 2,
        py: 1.25,
        borderTop: 1,
        borderColor: 'divider',
        '&:first-of-type': { borderTop: 0 },
      }}
    >
      <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
        <Typography variant="body1">{title}</Typography>
        {caption ? (
          <Typography variant="caption" color="text.secondary">
            {caption}
          </Typography>
        ) : null}
      </Stack>
      <Button size="small" variant="outlined" color="inherit" onClick={onList}>
        Add
      </Button>
      <Button size="small" variant="contained" onClick={onBought}>
        Bought it
      </Button>
    </Stack>
  );
}
