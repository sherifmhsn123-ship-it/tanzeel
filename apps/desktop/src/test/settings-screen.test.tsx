import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
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

async function openSettings() {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText(ar.nav.home);
  await user.click(screen.getByRole('button', { name: ar.nav.settings }));
  await screen.findByLabelText(ar.settings.downloadDir);
  return user;
}

describe('settings screen', () => {
  it('saves the download folder and theme locally', async () => {
    const user = await openSettings();

    const dirInput = screen.getByLabelText(ar.settings.downloadDir);
    await user.clear(dirInput);
    await user.type(dirInput, 'D:\\Downloads');
    await user.selectOptions(screen.getByLabelText(ar.settings.theme), 'dark');
    await user.click(screen.getByRole('button', { name: ar.settings.save }));

    expect(await screen.findByText(ar.settings.saved)).toBeInTheDocument();

    const stored = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored as string) as {
      downloadDir: string;
      theme: string;
    };
    expect(parsed.downloadDir).toBe('D:\\Downloads');
    expect(parsed.theme).toBe('dark');
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
  });

  it('rejects a relative folder path without saving', async () => {
    const user = await openSettings();

    const dirInput = screen.getByLabelText(ar.settings.downloadDir);
    await user.clear(dirInput);
    await user.type(dirInput, 'relative\\path');
    await user.click(screen.getByRole('button', { name: ar.settings.save }));

    expect(await screen.findByRole('alert')).toHaveTextContent(ar.settings.invalidDir);
    expect(window.localStorage.getItem(SETTINGS_STORAGE_KEY)).toBeNull();
  });

  it('applies a language chosen from the settings screen', async () => {
    const user = await openSettings();

    await user.selectOptions(screen.getByLabelText(ar.settings.language), 'en');

    expect(await screen.findByText(en.settings.saved)).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe('ltr'));
    expect(screen.getByRole('heading', { name: en.settings.title })).toBeInTheDocument();
  });
});
