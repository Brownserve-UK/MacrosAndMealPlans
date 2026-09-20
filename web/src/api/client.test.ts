import { describe, expect, it } from 'vitest';
import { ApiError, encodeCredential, ifMatch } from './client';

describe('api error', () => {
  it('recognises a revision conflict', () => {
    const error = new ApiError(409, {
      type: 'x',
      title: 'Revision conflict',
      status: 409,
      detail: 'changed',
      expected_revision: 1,
      actual_revision: 2,
    });
    expect(error.isConflict).toBe(true);
  });

  it('does not treat a duplicate as a revision conflict', () => {
    const error = new ApiError(409, {
      type: 'x',
      title: 'Already exists',
      status: 409,
      detail: 'taken',
    });
    expect(error.isConflict).toBe(false);
  });

  it('exposes field errors for a form', () => {
    const error = new ApiError(422, {
      type: 'x',
      title: 'Validation failed',
      status: 422,
      detail: 'bad',
      errors: [{ field: 'name', message: 'must not be blank' }],
    });
    expect(error.fieldErrors).toEqual({ name: 'must not be blank' });
  });
});

describe('request helpers', () => {
  it('quotes the revision as an entity tag', () => {
    expect(ifMatch(7)).toEqual({ 'If-Match': '"7"' });
  });

  it('encodes basic credentials', () => {
    expect(encodeCredential('admin', 'changeme')).toBe(`Basic ${btoa('admin:changeme')}`);
  });
});
