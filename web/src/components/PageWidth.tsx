import { createContext, useContext, useEffect } from 'react';

export type ContainerWidth = 'md' | 'xl';

export const SetContainerWidthContext = createContext<(width: ContainerWidth) => void>(() => {});

export function usePageContainerWidth(width: ContainerWidth) {
  const setContainerWidth = useContext(SetContainerWidthContext);
  useEffect(() => {
    setContainerWidth(width);
    return () => setContainerWidth('md');
  }, [width, setContainerWidth]);
}
