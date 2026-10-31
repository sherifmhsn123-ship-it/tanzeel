import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';
import i18n, { applyLanguage } from '../i18n';
import ar from '../i18n/ar.json';

beforeEach(async () => {
  window.localStorage.clear();
  await i18n.changeLanguage('ar');
  applyLanguage('ar');
});

describe('network isolation', () => {
  it('does not issue network requests while rendering the shell', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    render(<App />);
    await screen.findByText(ar.nav.home);

    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
