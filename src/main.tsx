import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app/App';
import './styles/tokens.css';
import './styles/fonts.css';
import './styles/base.css';

const root = document.getElementById('root');
if (!root) {
  throw new Error('index.html must contain a #root element');
}
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
