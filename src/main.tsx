import React from 'react';
import ReactDOM from 'react-dom/client';
// Global styles first so component stylesheets can refine the measured rules.
import './styles/tokens.css';
import './styles/measured.css';
import './styles/fonts.css';
import './styles/base.css';
import { App } from './app/App';

if (import.meta.env.DEV) {
  import('./dev/measure').then((m) => m.installMeasureBridge()).catch(() => undefined);
}

const root = document.getElementById('root');
if (!root) {
  throw new Error('index.html must contain a #root element');
}
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
