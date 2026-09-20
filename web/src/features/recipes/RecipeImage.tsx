import Box from '@mui/material/Box';
import { useEffect, useMemo } from 'react';
import { useRecipePhoto } from '../../api/queries';

export function RecipeImage({
  id,
  version,
  size,
  alt,
  sx,
}: {
  id: string;
  version: number;
  size: 'card' | 'hero';
  alt: string;
  sx?: object;
}) {
  const query = useRecipePhoto(id, size, version);
  const url = useMemo(() => query.data ? URL.createObjectURL(query.data) : null, [query.data]);

  useEffect(() => {
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [url]);

  if (!url) return null;
  return (
    <Box
      component="img"
      src={url}
      alt={alt}
      loading={size === 'card' ? 'lazy' : undefined}
      sx={sx}
    />
  );
}
