import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import i18n, { applyLanguage } from './i18n';
import { defaultSettings, loadSettings, saveSettings, type Settings } from './lib/settings';
import { applyTheme } from './lib/theme';
import AboutPage from './pages/AboutPage';
import HomePage from './pages/HomePage';
import SettingsPage from './pages/SettingsPage';

const tabs = ['home', 'settings', 'about'] as const;
type Tab = (typeof tabs)[number];

export default function App() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [activeTab, setActiveTab] = useState<Tab>('home');
  const [ready, setReady] = useState(false);
  const [languageSaving, setLanguageSaving] = useState(false);
  const [languageError, setLanguageError] = useState(false);

  useEffect(() => {
    let active = true;
    void loadSettings()
      .catch(() => ({ ...defaultSettings }))
      .then((loaded) => {
        if (active) {
          setSettings(loaded);
          setReady(true);
        }
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    void i18n.changeLanguage(settings.language).then(() => {
      if (!cancelled) {
        applyLanguage(settings.language);
      }
    });
    applyTheme(settings.theme);
    return () => {
      cancelled = true;
    };
  }, [settings.language, settings.theme]);

  const updateSettings = useCallback(async (next: Settings) => {
    await saveSettings(next);
    setSettings(next);
  }, []);

  const toggleLanguage = useCallback(async () => {
    if (languageSaving) {
      return;
    }

    const previous = settings;
    const next: Settings = {
      ...settings,
      language: settings.language === 'ar' ? 'en' : 'ar',
    };

    setLanguageSaving(true);
    setLanguageError(false);
    setSettings(next);
    try {
      await saveSettings(next);
    } catch {
      setSettings(previous);
      setLanguageError(true);
    } finally {
      setLanguageSaving(false);
    }
  }, [languageSaving, settings]);

  if (!ready) {
    return <div className="app-loading">{t('app.name')}</div>;
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="app-brand">
          <p className="app-name">{t('app.name')}</p>
          <p className="app-tagline">{t('app.tagline')}</p>
        </div>
        <button
          type="button"
          className="language-button"
          onClick={() => void toggleLanguage()}
          aria-label={t('nav.languageSwitchTo')}
          aria-busy={languageSaving}
          disabled={languageSaving}
        >
          {settings.language === 'ar' ? t('nav.languageCodeEnglish') : t('nav.languageCodeArabic')}
        </button>
      </header>

      {languageError && (
        <p className="app-error" role="alert">
          {t('settings.saveFailed')}
        </p>
      )}

      <nav className="app-nav" aria-label={t('app.name')}>
        {tabs.map((tab) => (
          <button
            key={tab}
            type="button"
            className={tab === activeTab ? 'nav-button nav-button-active' : 'nav-button'}
            aria-current={tab === activeTab ? 'page' : undefined}
            onClick={() => setActiveTab(tab)}
          >
            {t(`nav.${tab}`)}
          </button>
        ))}
      </nav>

      <main className="app-main">
        {activeTab === 'home' && <HomePage />}
        {activeTab === 'settings' && (
          <SettingsPage settings={settings} onSave={updateSettings} />
        )}
        {activeTab === 'about' && <AboutPage />}
      </main>

      <footer className="app-footer">
        <span>{t('app.status')}</span>
      </footer>
    </div>
  );
}
