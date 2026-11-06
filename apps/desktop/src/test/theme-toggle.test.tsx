import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';
import i18n, { applyLanguage } from '../i18n';
import ar from '../i18n/ar.json';
import en from '../i18n/en.json';
import { SETTINGS_STORAGE_KEY, defaultSettings } from '../lib/settings';

beforeEach(async () => {
  window.localStorage.clear();
  await i18n.changeLanguage('ar');
  applyLanguage('ar');
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function readStoredTheme(): string | undefined {
  const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
  return raw === null ? undefined : (JSON.parse(raw) as { theme: string }).theme;
}

describe('header theme toggle', () => {
  it('toggles light and dark from the header and persists the choice', async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText(ar.nav.home);

    await user.click(screen.getByRole('button', { name: ar.nav.themeSwitchToDark }));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
    expect(readStoredTheme()).toBe('dark');
    expect(screen.getByRole('button', { name: ar.nav.themeSwitchToLight })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: ar.nav.themeSwitchToLight }));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
    expect(readStoredTheme()).toBe('light');
    expect(screen.getByRole('button', { name: ar.nav.themeSwitchToDark })).toBeInTheDocument();
  });

  it('keeps the chosen theme after remounting', async () => {
    const user = userEvent.setup();
    const first = render(<App />);
    await screen.findByText(ar.nav.home);

    await user.click(screen.getByRole('button', { name: ar.nav.themeSwitchToDark }));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
    first.unmount();

    render(<App />);
    await screen.findByText(ar.nav.home);
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
    expect(screen.getByRole('button', { name: ar.nav.themeSwitchToLight })).toBeInTheDocument();
  });

  it('uses the system preference when the saved theme is system', async () => {
    vi.stubGlobal('matchMedia', (query: string) => ({
      matches: query === '(prefers-color-scheme: dark)',
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));
    window.localStorage.setItem(
      SETTINGS_STORAGE_KEY,
      JSON.stringify({ ...defaultSettings, theme: 'system' }),
    );

    const user = userEvent.setup();
    render(<App />);
    await screen.findByText(ar.nav.home);

    await user.click(await screen.findByRole('button', { name: ar.nav.themeSwitchToLight }));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
    expect(readStoredTheme()).toBe('light');
  });

  it('reverts and reports when the theme cannot be saved', async () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('storage unavailable');
    });
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText(ar.nav.home);

    await user.click(screen.getByRole('button', { name: ar.nav.themeSwitchToDark }));

    expect(await screen.findByRole('alert')).toHaveTextContent(ar.settings.saveFailed);
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(screen.getByRole('button', { name: ar.nav.themeSwitchToDark })).toBeInTheDocument();
  });

  it('updates the toggle labels when the language changes', async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText(ar.nav.home);

    await user.click(screen.getByRole('button', { name: ar.nav.languageSwitchTo }));

    expect(await screen.findByRole('button', { name: en.nav.themeSwitchToDark })).toBeInTheDocument();
  });
});
