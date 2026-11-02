import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';
import i18n, { applyLanguage } from '../i18n';
import ar from '../i18n/ar.json';
import en from '../i18n/en.json';
import { SETTINGS_STORAGE_KEY } from '../lib/settings';

beforeEach(async () => {
  window.localStorage.clear();
  await i18n.changeLanguage('ar');
  applyLanguage('ar');
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('language switching', () => {
  it('switches instantly, updates direction, and persists the choice', async () => {
    const user = userEvent.setup();
    const first = render(<App />);

    expect(await screen.findByText(ar.nav.home)).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe('rtl'));

    await user.click(screen.getByRole('button', { name: ar.nav.languageSwitchTo }));

    expect(await screen.findByText(en.nav.home)).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe('ltr'));
    expect(document.documentElement.lang).toBe('en');

    await waitFor(() => {
      const stored = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
      expect(stored).not.toBeNull();
      expect((JSON.parse(stored as string) as { language: string }).language).toBe('en');
    });

    first.unmount();
    render(<App />);

    expect(await screen.findByText(en.nav.home)).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe('ltr'));
  });

  it('keeps Arabic as the default direction when no settings exist', async () => {
    render(<App />);
    expect(await screen.findByText(ar.nav.home)).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe('rtl'));
    expect(document.documentElement.lang).toBe('ar');
  });

  it('reverts and reports when the header language choice cannot be saved', async () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('storage unavailable');
    });
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText(ar.nav.home)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: ar.nav.languageSwitchTo }));

    expect(await screen.findByRole('alert')).toHaveTextContent(ar.settings.saveFailed);
    expect(screen.getByText(ar.nav.home)).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe('rtl'));
    expect(setItem).toHaveBeenCalledWith(SETTINGS_STORAGE_KEY, expect.any(String));
  });
});
