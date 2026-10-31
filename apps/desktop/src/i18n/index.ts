import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import ar from './ar.json';
import en from './en.json';
import type { Language } from '../lib/settings';

export const resources = {
  ar: { translation: ar },
  en: { translation: en },
} as const;

void i18n.use(initReactI18next).init({
  resources,
  lng: 'ar',
  fallbackLng: 'en',
  supportedLngs: ['ar', 'en'],
  interpolation: { escapeValue: false },
});

export function applyLanguage(language: Language): void {
  document.documentElement.lang = language;
  document.documentElement.dir = language === 'ar' ? 'rtl' : 'ltr';
  document.title = i18n.t('app.name');
}

export default i18n;
