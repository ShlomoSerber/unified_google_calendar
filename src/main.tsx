import React from 'react';
import ReactDOM from 'react-dom/client';
// Global styles first so component stylesheets can refine them.
import './styles/tokens.css';
import './styles/typescale.css';
import './styles/fonts.css';
import './styles/base.css';
// Registers the <md-*> elements before any component renders one.
import './m3/register';
import { App } from './app/App';

const root = document.getElementById('root');
if (!root) {
  throw new Error('index.html must contain a #root element');
}
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
