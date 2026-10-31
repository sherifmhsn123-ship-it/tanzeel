import { useTranslation } from 'react-i18next';

export default function AboutPage() {
  const { t } = useTranslation();

  return (
    <section className="page" aria-labelledby="about-title">
      <h1 id="about-title">{t('about.title')}</h1>
      <p className="page-description">{t('about.description')}</p>
      <dl className="info-list">
        <div>
          <dt>{t('about.version')}</dt>
          <dd>0.1.0</dd>
        </div>
        <div>
          <dt>{t('about.license')}</dt>
          <dd>{t('about.licenseValue')}</dd>
        </div>
        <div>
          <dt>{t('about.status')}</dt>
          <dd>{t('about.statusValue')}</dd>
        </div>
      </dl>
    </section>
  );
}
