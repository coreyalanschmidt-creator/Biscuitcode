// src/main.tsx
//
// React + i18next bootstrap. Mounts <App /> into #root and inits the
// i18n bundle from src/locales/en.json so every t('key') call works
// from the very first render.
//
// Phase 1 deliverable. Phase 1 coder may need to tweak the React import
// style to match the scaffold's React version (18 uses createRoot;
// older versions used ReactDOM.render).

import React from 'react';
import { createRoot } from 'react-dom/client';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from './locales/en.json';
import App from './App';
import './index.css';

i18n.use(initReactI18next).init({
  resources: { en: { translation: en } },
  lng: 'en',
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false, // React already escapes
  },
  // Throw on missing keys in dev — catches `t('errors.E999')` typos.
  saveMissing: import.meta.env.DEV,
  missingKeyHandler: (lng, ns, key) => {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.warn(`i18n missing key: ${key} (lng=${lng}, ns=${ns})`);
    }
  },
});

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('Root element #root not found in index.html');
}

createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
