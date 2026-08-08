import { describe, expect, it } from 'vitest';
import { normalizeError } from './error';

describe('normalizeError', () => {
  it('preserves structured Tauri errors', () => {
    expect(
      normalizeError({ code: 'INVALID_BASE_REF', message: 'Missing ref', detail: 'origin/main' })
    ).toEqual({ code: 'INVALID_BASE_REF', message: 'Missing ref', detail: 'origin/main' });
  });

  it('parses serialized structured errors', () => {
    expect(normalizeError('{"code":"AI_KEY_MISSING","message":"Set a key"}')).toEqual({
      code: 'AI_KEY_MISSING',
      message: 'Set a key',
      detail: undefined
    });
  });
});
