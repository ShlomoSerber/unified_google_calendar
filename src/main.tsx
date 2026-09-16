import React from 'react';
import ReactDOM from 'react-dom/client';
// Global styles first so component stylesheets can refine them. legacy.css and measured.css
// keep the unmigrated components alive until M5 of the Material 3 transition (docs/12).
import './styles/legacy.css';
import './styles/measured.css';
import './styles/tokens.css';
import './styles/typescale.css';
import './styles/fonts.css';
import './styles/base.css';
// Registers the <md-*> elements before any component renders one.
import './m3/register';
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
