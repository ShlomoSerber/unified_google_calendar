import { useEffect, useState } from 'react';
import { installTooltips, ms, usePresence, type TooltipState } from '../lib/motion';
import { MOTION } from '../styles/legacy-motion';

// Component 21 (docs/design/measurements/tooltip-light.json): the tooltip Google shows under a
// button 540 ms after the pointer arrives, centred on it. The DOM mirrors the dump (root, box,
// label); src/lib/motion.ts decides when it shows and which button it belongs to.

const isOpen = (t: TooltipState | null) => t !== null;
const exitMs = () => ms(MOTION.tooltip_hide_duration);

export function Tooltip() {
  const [tip, setTip] = useState<TooltipState | null>(null);
  useEffect(() => installTooltips(setTip), []);
  const { shown, exiting } = usePresence(tip, isOpen, exitMs);
  if (!shown) return null;
  return (
    <div className={exiting ? 'tooltip-root tooltip-exit' : 'tooltip-root tooltip-enter'} style={{ left: shown.x, top: shown.y }}>
      <div className="tooltip-box">
        <span className="tooltip-label">{shown.text}</span>
      </div>
    </div>
  );
}
