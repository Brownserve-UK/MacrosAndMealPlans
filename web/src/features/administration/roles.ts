import type { Role } from '../../api/client';

export const ROLES: { value: Role; label: string; hint: string }[] = [
  { value: 'admin', label: 'Admin', hint: 'Everything, including private data' },
  { value: 'household_manager', label: 'Household manager', hint: 'Manages people and food' },
  { value: 'basic_user', label: 'Basic user', hint: 'Their own plans and the catalogue' },
  { value: 'nutritionist', label: 'Nutritionist', hint: 'Only what they are given access to' },
];

export function roleLabel(role: Role): string {
  return ROLES.find((r) => r.value === role)?.label ?? role;
}

export function describeRoles(roles: Role[]): string {
  if (roles.length === 0) return 'No access';
  return roles.map(roleLabel).join(', ');
}
