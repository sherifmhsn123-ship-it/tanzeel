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
    const next: Settings = {
      ...settings,
      language: settings.language === 'ar' ? 'en' : 'ar',
    };
    setSettings(next);
    try {
      await saveSettings(next);
    } catch {
      // Keep the interface responsive; the settings screen reports save errors.
    }
  }, [settings]);

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
        >
          {settings.language === 'ar' ? t('nav.languageCodeEnglish') : t('nav.languageCodeArabic')}
        </button>
      </header>

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
