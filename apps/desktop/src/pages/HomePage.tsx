import { useTranslation } from 'react-i18next';

export default function HomePage() {
  const { t } = useTranslation();

  return (
    <section className="page" aria-labelledby="home-title">
      <h1 id="home-title">{t('home.title')}</h1>
      <p className="page-description">{t('home.description')}</p>
      <div className="empty-state">
        <h2>{t('home.emptyTitle')}</h2>
        <p>{t('home.emptyBody')}</p>
      </div>
      <p className="phase-note">{t('home.phaseNote')}</p>
    </section>
  );
}
