import { invoke } from '@tauri-apps/api/core';

export type Language = 'ar' | 'en';
export type Theme = 'light' | 'dark' | 'system';

export interface Settings {
  language: Language;
  theme: Theme;
  downloadDir: string;
}

export const SETTINGS_STORAGE_KEY = 'tanzeel.settings.v1';

export const defaultSettings: Settings = {
  language: 'ar',
  theme: 'system',
  downloadDir: '',
};

const MAX_DOWNLOAD_DIR_LENGTH = 1024;

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function validateSettings(value: Settings): string | null {
  if (value.language !== 'ar' && value.language !== 'en') {
    return 'language';
  }
  if (value.theme !== 'light' && value.theme !== 'dark' && value.theme !== 'system') {
    return 'theme';
  }
  if (typeof value.downloadDir !== 'string') {
    return 'downloadDir';
  }
  const dir = value.downloadDir;
  if (Array.from(dir).length > MAX_DOWNLOAD_DIR_LENGTH) {
    return 'downloadDir';
  }
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f-\u009f]/.test(dir)) {
    return 'downloadDir';
  }
  if (dir.length === 0) {
    return null;
  }
  if (dir.trim() !== dir) {
    return 'downloadDir';
  }
  const isWindowsAbsolute = /^[A-Za-z]:[\\/]/.test(dir) || /^\\\\[^\\]/.test(dir);
  if (!isWindowsAbsolute) {
    return 'downloadDir';
  }
  return null;
}

export async function loadSettings(): Promise<Settings> {
  if (!isTauriRuntime()) {
    try {
      const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
      if (raw === null) {
        return { ...defaultSettings };
      }
      const parsed = JSON.parse(raw) as Settings;
      return validateSettings(parsed) === null ? parsed : { ...defaultSettings };
    } catch {
      return { ...defaultSettings };
    }
  }
  return invoke<Settings>('get_settings');
}

export async function saveSettings(settings: Settings): Promise<void> {
  const invalid = validateSettings(settings);
  if (invalid !== null) {
    throw new Error(`invalid-settings:${invalid}`);
  }
  if (!isTauriRuntime()) {
    window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
    return;
  }
  await invoke('save_settings', { settings });
}
