import { dismissNotice } from '../lib/notice';
import { ms, usePresence } from '../lib/motion';
import { useUi } from '../state/ui';
import { MOTION } from '../styles/motion';
import './Snackbar.css';

// Snackbar (docs/11 section 7): the bar that slides up from the bottom edge while a write runs
// ("Saving...") and when it finishes ("Event saved"), with a close button. No "Undo" action.

type Notice = { text: string; busy: boolean } | null;
const isOpen = (n: Notice) => n !== null;
const exitMs = () => ms(MOTION.snackbar_slide_duration);

export function Snackbar() {
  const notice = useUi((s) => s.notice);
  const { shown, exiting } = usePresence(notice, isOpen, exitMs);
  if (!shown) return null;
  return (
    <div className={exiting ? 'snackbar snackbar-exit' : 'snackbar snackbar-enter'} role="status" aria-live="polite">
      <span className="snackbar-label md-typescale-body-medium">{shown.text}</span>
      <md-icon-button aria-label="Close" onclick={dismissNotice}>
        <md-icon>close</md-icon>
      </md-icon-button>
    </div>
  );
}
