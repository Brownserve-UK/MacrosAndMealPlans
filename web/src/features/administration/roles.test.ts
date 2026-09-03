import { describe, expect, it } from 'vitest';
import { ROLES, describeRoles, roleLabel } from './roles';

describe('roles', () => {
  it('labels every role the API can return', () => {
    for (const role of ROLES) {
      expect(roleLabel(role.value)).toBe(role.label);
    }
  });

  it('falls back to the raw code for an unknown role', () => {
    expect(roleLabel('galactic_overlord' as never)).toBe('galactic_overlord');
  });

  it('joins multiple roles', () => {
    expect(describeRoles(['admin', 'basic_user'])).toBe('Admin, Basic user');
  });

  it('says so when an account has no roles', () => {
    expect(describeRoles([])).toBe('No access');
  });

  it('offers admin first, since that is the one that matters most', () => {
    expect(ROLES[0]?.value).toBe('admin');
  });
});
