import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import App from '../App';
import i18n, { applyLanguage } from '../i18n';
import ar from '../i18n/ar.json';
import en from '../i18n/en.json';
import { defaultSettings, SETTINGS_STORAGE_KEY, type Language } from '../lib/settings';

beforeEach(async () => {
  window.localStorage.clear();
  await i18n.changeLanguage('ar');
  applyLanguage('ar');
});

describe('main screens in both writing directions', () => {
  it.each([
    { language: 'ar' as Language, direction: 'rtl', copy: ar },
    { language: 'en' as Language, direction: 'ltr', copy: en },
  ])('renders every main screen in $direction', async ({ language, direction, copy }) => {
    window.localStorage.setItem(
      SETTINGS_STORAGE_KEY,
      JSON.stringify({ ...defaultSettings, language }),
    );

    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByRole('heading', { name: copy.home.title })).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dir).toBe(direction));

    await user.click(screen.getByRole('button', { name: copy.nav.settings }));
    expect(screen.getByRole('heading', { name: copy.settings.title })).toBeInTheDocument();
    expect(document.documentElement.dir).toBe(direction);

    await user.click(screen.getByRole('button', { name: copy.nav.about }));
    expect(screen.getByRole('heading', { name: copy.about.title })).toBeInTheDocument();
    expect(document.documentElement.dir).toBe(direction);
  });
});
