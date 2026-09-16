import { useCallback, useEffect, useRef, useState, type FocusEvent, type PointerEvent, type ReactNode } from 'react';
import { ms, usePresence } from '../lib/motion';
import { LAYOUT, layoutNumber } from '../styles/layout';
import { MOTION } from '../styles/motion';
import './Tooltip.css';

// Plain tooltip (docs/11 section 7): wraps a control, waits tooltip_delay after the pointer
// arrives (or focus lands), then draws the text under the control, centred. Only one tooltip
// is visible at a time: showing one hides the previous. Leaving, pressing or blurring the
// window hides it.

interface Point {
  x: number;
  y: number;
}

/** The tooltip on screen, so showing another one hides it first. */
let current: { owner: object; hide: () => void } | null = null;
const isOpen = (p: Point | null) => p !== null;
const exitMs = () => ms(MOTION.tooltip_hide_duration);

export interface TooltipProps {
  text: string;
  children: ReactNode;
}

export function Tooltip({ text, children }: TooltipProps) {
  const anchor = useRef<HTMLSpanElement>(null);
  const timer = useRef(0);
  const [point, setPoint] = useState<Point | null>(null);
  const { shown, exiting } = usePresence(point, isOpen, exitMs);

  const [owner] = useState(() => ({}));
  const hide = useCallback(() => {
    window.clearTimeout(timer.current);
    setPoint(null);
    if (current?.owner === owner) current = null;
  }, [owner]);
  const show = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      const r = anchor.current?.getBoundingClientRect();
      if (!r) return;
      if (current && current.owner !== owner) current.hide();
      current = { owner, hide };
      setPoint({ x: r.left + r.width / 2, y: r.bottom + layoutNumber(LAYOUT.tooltip_gap) });
    }, ms(MOTION.tooltip_delay));
  };
  // Moving between the control and its own children is not a leave.
  const leave = (e: PointerEvent | FocusEvent) => {
    const to = e.relatedTarget;
    if (to instanceof Node && anchor.current?.contains(to)) return;
    hide();
  };
  useEffect(() => {
    document.addEventListener('pointerdown', hide, true);
    window.addEventListener('blur', hide);
    return () => {
      document.removeEventListener('pointerdown', hide, true);
      window.removeEventListener('blur', hide);
      hide();
    };
  }, [hide]);

  return (
    <span className="tooltip-anchor" ref={anchor} onPointerEnter={show} onPointerLeave={leave} onFocus={show} onBlur={leave}>
      {children}
      {shown ? (
        <span className={`tooltip md-typescale-body-small${exiting ? ' tooltip-exit' : ' tooltip-enter'}`} role="tooltip" style={{ left: shown.x, top: shown.y }}>
          {text}
        </span>
      ) : null}
    </span>
  );
}
