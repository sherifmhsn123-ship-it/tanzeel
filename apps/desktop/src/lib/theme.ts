import type { Theme } from './settings';

export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (theme === 'system') {
    delete root.dataset.theme;
    return;
  }
  root.dataset.theme = theme;
}
