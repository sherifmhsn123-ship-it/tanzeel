import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(() => {
  cleanup();
  window.localStorage.clear();
  const root = document.documentElement;
  root.dir = 'rtl';
  root.lang = 'ar';
  delete root.dataset.theme;
});
