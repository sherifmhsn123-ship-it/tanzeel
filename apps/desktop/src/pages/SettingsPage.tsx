import { useState, type ChangeEvent, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import type { Language, Settings, Theme } from '../lib/settings';

interface SettingsPageProps {
  settings: Settings;
  onSave: (next: Settings) => Promise<void>;
}

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

export default function SettingsPage({ settings, onSave }: SettingsPageProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<Settings>(settings);
  const [syncedSettings, setSyncedSettings] = useState<Settings>(settings);
  const [state, setState] = useState<SaveState>('idle');
  const [errorKey, setErrorKey] = useState<string | null>(null);

  if (settings !== syncedSettings) {
    setSyncedSettings(settings);
    setDraft(settings);
  }

  const persist = async (next: Settings): Promise<boolean> => {
    setState('saving');
    setErrorKey(null);
    try {
      await onSave(next);
      setState('saved');
      return true;
    } catch (error) {
      const invalid = error instanceof Error && error.message.startsWith('invalid-settings:');
      setErrorKey(invalid ? 'settings.invalidDir' : 'settings.saveFailed');
      setState('error');
      return false;
    }
  };

  const changeLanguage = (event: ChangeEvent<HTMLSelectElement>) => {
    const next: Settings = { ...draft, language: event.target.value as Language };
    setDraft(next);
    void persist(next).then((saved) => {
      if (!saved) {
        setDraft(settings);
      }
    });
  };

  const changeTheme = (event: ChangeEvent<HTMLSelectElement>) => {
    setDraft({ ...draft, theme: event.target.value as Theme });
    setState('idle');
  };

  const changeDownloadDir = (event: ChangeEvent<HTMLInputElement>) => {
    setDraft({ ...draft, downloadDir: event.target.value });
    setState('idle');
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void persist(draft);
  };

  return (
    <section className="page" aria-labelledby="settings-title">
      <h1 id="settings-title">{t('settings.title')}</h1>
      <p className="page-description">{t('settings.description')}</p>

      <form className="settings-form" onSubmit={submit}>
        <div className="field">
          <label htmlFor="language">{t('settings.language')}</label>
          <select id="language" value={draft.language} onChange={changeLanguage}>
            <option value="ar">{t('settings.languageArabic')}</option>
            <option value="en">{t('settings.languageEnglish')}</option>
          </select>
        </div>

        <div className="field">
          <label htmlFor="theme">{t('settings.theme')}</label>
          <select id="theme" value={draft.theme} onChange={changeTheme}>
            <option value="system">{t('settings.themeSystem')}</option>
            <option value="light">{t('settings.themeLight')}</option>
            <option value="dark">{t('settings.themeDark')}</option>
          </select>
        </div>

        <div className="field">
          <label htmlFor="downloadDir">{t('settings.downloadDir')}</label>
          <input
            id="downloadDir"
            type="text"
            dir="ltr"
            value={draft.downloadDir}
            placeholder={t('settings.downloadDirPlaceholder')}
            aria-describedby="downloadDir-hint"
            onChange={changeDownloadDir}
          />
          <p id="downloadDir-hint" className="field-hint">
            {t('settings.downloadDirHint')}
          </p>
        </div>

        <div className="form-actions">
          <button type="submit" disabled={state === 'saving'}>
            {state === 'saving' ? t('settings.saving') : t('settings.save')}
          </button>
          {state === 'saved' && (
            <p className="form-status" role="status">
              {t('settings.saved')}
            </p>
          )}
          {state === 'error' && errorKey !== null && (
            <p className="form-error" role="alert">
              {t(errorKey)}
            </p>
          )}
        </div>
      </form>
    </section>
  );
}
