import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';
import './i18n';
import App from './App';

const container = document.getElementById('root');
if (container === null) {
  throw new Error('root container is missing');
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
