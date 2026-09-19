import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { Member, User } from '../client';
import type { components } from '../schema';
import { householdKeys } from '../keys';
import type { MemberListParams, UserListParams } from '../keys';

export function useMembers(params: MemberListParams) {
  return useQuery({
    queryKey: householdKeys.memberList(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/members', { params: { query: params } })),
  });
}

export function useMember(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: householdKeys.memberDetail(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/members/{id}', { params: { path: { id } } })),
  });
}

export function useCreateMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateMemberRequest']) =>
      unwrap(await client.POST('/api/v1/members', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: householdKeys.members() }),
  });
}

export function useUpdateMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateMemberRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/members/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: Member) => {
      qc.setQueryData(householdKeys.memberDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: householdKeys.members() });
    },
  });
}

export function useSetMemberArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/members/{id}/archive' as const)
        : ('/api/v1/members/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: Member) => {
      qc.setQueryData(householdKeys.memberDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: householdKeys.members() });
    },
  });
}

export function useSetMemberAccount() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; userId: string | null }) => {
      const params = { path: { id: input.id }, header: ifMatch(input.revision) };
      return input.userId === null
        ? unwrap(await client.DELETE('/api/v1/members/{id}/account', { params }))
        : unwrap(
            await client.PUT('/api/v1/members/{id}/account', {
              params,
              body: { user_id: input.userId },
            }),
          );
    },
    onSuccess: (updated: Member) => {
      qc.setQueryData(householdKeys.memberDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: householdKeys.members() });
      void qc.invalidateQueries({ queryKey: householdKeys.users() });
    },
  });
}

export function useMemberAccess(id: string) {
  return useQuery({
    queryKey: householdKeys.memberAccess(id),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/members/{id}/access', { params: { path: { id } } })),
  });
}

export function useGrantMemberAccess() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      body: components['schemas']['GrantAccessRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/members/{id}/access', {
          params: { path: { id: input.id } },
          body: input.body,
        }),
      ),
    onSuccess: (_data, input) =>
      qc.invalidateQueries({ queryKey: householdKeys.memberAccess(input.id) }),
  });
}

export function useRevokeMemberAccess() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      userId: string;
      scope: components['schemas']['AccessScope'];
    }) =>
      unwrap(
        await client.DELETE('/api/v1/members/{id}/access/{user_id}/{scope}', {
          params: { path: { id: input.id, user_id: input.userId, scope: input.scope } },
        }),
      ),
    onSuccess: (_data, input) =>
      qc.invalidateQueries({ queryKey: householdKeys.memberAccess(input.id) }),
  });
}

export function useUsers(params: UserListParams) {
  return useQuery({
    queryKey: householdKeys.userList(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/users', { params: { query: params } })),
  });
}

export function useCreateUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateUserRequest']) =>
      unwrap(await client.POST('/api/v1/users', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: householdKeys.users() }),
  });
}

export function useSetUserRoles() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      roles: components['schemas']['Role'][];
    }) =>
      unwrap(
        await client.PUT('/api/v1/users/{id}/roles', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: { roles: input.roles },
        }),
      ),
    onSuccess: (updated: User) => {
      qc.setQueryData(householdKeys.userDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: householdKeys.users() });
    },
  });
}

export function useSetUserArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/users/{id}/archive' as const)
        : ('/api/v1/users/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: User) => {
      qc.setQueryData(householdKeys.userDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: householdKeys.users() });
      void qc.invalidateQueries({ queryKey: householdKeys.members() });
    },
  });
}
