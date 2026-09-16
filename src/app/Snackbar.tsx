import { dismissNotice } from '../lib/notice';
import { ms, usePresence } from '../lib/motion';
import { useUi } from '../state/ui';
import { MOTION } from '../styles/legacy-motion';
import './Snackbar.css';

// Component 24 (docs/design/measurements/snackbar-save-light.json): the bar Google slides up
// from the bottom edge while a write runs and when it finishes. The DOM mirrors the dump: text
// cell and a close button; Google's "Undo" is out of scope.

const CLOSE = 'M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z';
type Notice = { text: string; busy: boolean } | null;
const isOpen = (n: Notice) => n !== null;
const exitMs = () => ms(MOTION.snackbar_slide_duration);

export function Snackbar() {
  const notice = useUi((s) => s.notice);
  const { shown, exiting } = usePresence(notice, isOpen, exitMs);
  if (!shown) return null;
  return (
    <div className={exiting ? 'snackbar-root snackbar-exit' : 'snackbar-root snackbar-enter'} role="status" aria-live="polite">
      <div className="snackbar-row">
        <div className="snackbar-text">{shown.text}</div>
        <div className="snackbar-close-cell">
          <div className="snackbar-close">
            <button className="snackbar-close-button" aria-label="Close" type="button" onClick={dismissNotice}>
              <span className="snackbar-close-span">
                <span className="snackbar-close-icon-box">
                  <svg className="snackbar-close-icon" viewBox="0 0 24 24" focusable="false">
                    <path className="snackbar-close-path" d={CLOSE} />
                  </svg>
                </span>
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
