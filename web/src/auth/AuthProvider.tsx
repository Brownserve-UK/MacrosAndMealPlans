import { createContext, use, useCallback, useMemo, useState, type ReactNode } from 'react';
import { ApiError, client, encodeCredential, hasCredential, setCredential, unwrap } from '../api/client';
import type { Principal } from '../api/client';

type AuthState = {
  principal: Principal | null;
  signIn: (username: string, password: string) => Promise<void>;
  signOut: () => void;
  restore: () => Promise<void>;
  restored: boolean;
};

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [principal, setPrincipal] = useState<Principal | null>(null);
  const [restored, setRestored] = useState(!hasCredential());

  const signIn = useCallback(async (username: string, password: string) => {
    setCredential(encodeCredential(username, password));
    try {
      const me = unwrap(await client.POST('/api/v1/auth/session', {}));
      setPrincipal(me);
    } catch (error) {
      setCredential(null);
      setPrincipal(null);
      throw error;
    }
  }, []);

  const signOut = useCallback(() => {
    setCredential(null);
    setPrincipal(null);
  }, []);

  const restore = useCallback(async () => {
    if (!hasCredential()) {
      setRestored(true);
      return;
    }
    try {
      setPrincipal(unwrap(await client.GET('/api/v1/auth/me', {})));
    } catch (error) {
      if (error instanceof ApiError && error.isUnauthorized) setCredential(null);
    } finally {
      setRestored(true);
    }
  }, []);

  const value = useMemo(
    () => ({ principal, signIn, signOut, restore, restored }),
    [principal, signIn, signOut, restore, restored],
  );

  return <AuthContext value={value}>{children}</AuthContext>;
}

export function useAuth(): AuthState {
  const context = use(AuthContext);
  if (!context) throw new Error('useAuth must be used inside an AuthProvider');
  return context;
}
