import { describe, expect, it } from 'vitest';
import { defaultSettings, validateSettings, type Settings } from '../lib/settings';

const valid: Settings = { ...defaultSettings, downloadDir: 'D:\\Downloads' };

describe('settings validation', () => {
  it('accepts empty, drive-absolute, and UNC paths', () => {
    expect(validateSettings({ ...valid, downloadDir: '' })).toBeNull();
    expect(validateSettings(valid)).toBeNull();
    expect(validateSettings({ ...valid, downloadDir: '\\\\server\\share' })).toBeNull();
  });

  it('rejects relative, drive-relative, and whitespace-only paths', () => {
    expect(validateSettings({ ...valid, downloadDir: 'Downloads' })).toBe('downloadDir');
    expect(validateSettings({ ...valid, downloadDir: 'C:Downloads' })).toBe('downloadDir');
    expect(validateSettings({ ...valid, downloadDir: '   ' })).toBe('downloadDir');
  });

  it('rejects surrounding whitespace and control characters', () => {
    expect(validateSettings({ ...valid, downloadDir: ' D:\\Downloads ' })).toBe('downloadDir');
    expect(validateSettings({ ...valid, downloadDir: 'D:\\Down\0loads' })).toBe('downloadDir');
    expect(validateSettings({ ...valid, downloadDir: 'D:\\Down\u009floads' })).toBe('downloadDir');
  });

  it('rejects overlong paths using Unicode scalar counts', () => {
    const tooLong = `C:\\${'a'.repeat(1022)}`;
    expect(validateSettings({ ...valid, downloadDir: tooLong })).toBe('downloadDir');
  });

  it('rejects unknown languages and themes', () => {
    expect(validateSettings({ ...valid, language: 'fr' as Settings['language'] })).toBe('language');
    expect(validateSettings({ ...valid, theme: 'blue' as Settings['theme'] })).toBe('theme');
  });
});
