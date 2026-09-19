import CssBaseline from '@mui/material/CssBaseline';
import { ThemeProvider } from '@mui/material/styles';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { ApiError } from './api/client';
import { AuthProvider, useAuth } from './auth/AuthProvider';
import { LoginPage } from './features/LoginPage';
import { Loading } from './components/States';
import { router } from './routes/router';
import { theme } from './theme/theme';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: (failureCount, error) => {
        if (error instanceof ApiError && error.status < 500) return false;
        return failureCount < 2;
      },
    },
  },
});

function Gate() {
  const { principal, restore, restored } = useAuth();

  useEffect(() => {
    void restore();
  }, [restore]);

  if (!restored) return <Loading label="Loading" />;
  if (!principal) return <LoginPage />;
  return <RouterProvider router={router} />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider theme={theme} defaultMode="system">
      <CssBaseline />
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <Gate />
        </AuthProvider>
      </QueryClientProvider>
    </ThemeProvider>
  </StrictMode>,
);
