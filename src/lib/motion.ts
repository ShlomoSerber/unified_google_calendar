// Motion helpers (docs/04 section 10, tokens.json component.motion, measurements/animations.md):
// keep an element mounted while its exit animation plays, and the Material ripple Google's
// buttons run on press. Timings come from the tokens; nothing here is typed by hand.
import { useEffect, useState } from 'react';
import { LAYOUT, layoutNumber } from '../styles/legacy-layout';
import { MOTION } from '../styles/legacy-motion';

/** "150ms" → 150. */
export const ms = (v: string): number => parseFloat(v);

/**
 * Presence with an exit delay: while `isOpen(value)` the current value is shown at once; when it
 * closes, the last open value stays rendered for `exitMs(last)` with `exiting` true, so the CSS
 * exit animation can play before the element unmounts.
 */
export function usePresence<T>(value: T, isOpen: (v: T) => boolean, exitMs: (last: T) => number): { shown: T; exiting: boolean } {
  const [state, setState] = useState<{ shown: T; exiting: boolean }>({ shown: value, exiting: false });
  // Derived during render so the exit class is committed in the same frame the dialog closes.
  if (isOpen(value)) {
    if (state.shown !== value || state.exiting) setState({ shown: value, exiting: false });
  } else if (state.shown !== value && !state.exiting) {
    setState(isOpen(state.shown) ? { shown: state.shown, exiting: true } : { shown: value, exiting: false });
  }
  const { shown, exiting } = state;
  useEffect(() => {
    if (!exiting) return undefined;
    const t = window.setTimeout(() => setState({ shown: value, exiting: false }), exitMs(shown));
    return () => window.clearTimeout(t);
    // `value` is the closed value while exiting; exitMs is a module-level constant function.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [exiting, shown]);
  return state;
}

/** Element whose state layer (`.ugc-state` child) hosts the ripple for a pointer event target. */
function rippleHost(target: EventTarget | null): { host: HTMLElement; layer: HTMLElement } | null {
  if (!(target instanceof Element)) return null;
  let el: Element | null = target;
  while (el && el !== document.body) {
    const layer = el.querySelector(':scope > .ugc-state');
    if (layer instanceof HTMLElement && el instanceof HTMLElement) return { host: el, layer };
    el = el.parentElement;
  }
  return null;
}

/**
 * Google's press ripple (tokens.json component.motion.ripple_*): a circle of
 * floor(max(w, h) × 0.2) px starts under the pointer and grows to cover the button in 450 ms,
 * moving to the centre, then fades with the pressed tint. One document-level listener serves
 * every button that carries a `.ugc-state` span.
 */
export function installRipples(): () => void {
  const onDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    const found = rippleHost(e.target);
    if (!found) return;
    const { host, layer } = found;
    if (host.matches(':disabled, [aria-disabled="true"]')) return;
    const rect = layer.getBoundingClientRect();
    const w = rect.width;
    const h = rect.height;
    const maxDim = Math.max(w, h);
    const initial = Math.floor(maxDim * parseFloat(MOTION.ripple_initial_scale));
    if (initial <= 0) return;
    const softEdge = Math.max(parseFloat(MOTION.ripple_soft_edge_ratio) * maxDim, layoutNumber(MOTION.ripple_soft_edge_min));
    const scale = (Math.hypot(w, h) + layoutNumber(MOTION.ripple_padding) + softEdge) / initial;
    const startX = e.clientX - rect.left - initial / 2;
    const startY = e.clientY - rect.top - initial / 2;
    const endX = (w - initial) / 2;
    const endY = (h - initial) / 2;
    const ripple = document.createElement('span');
    ripple.className = 'ugc-ripple';
    ripple.style.width = `${initial}px`;
    ripple.style.height = `${initial}px`;
    layer.appendChild(ripple);
    ripple.animate(
      [
        { transform: `translate(${startX}px, ${startY}px) scale(1)` },
        { transform: `translate(${endX}px, ${endY}px) scale(${scale})` },
      ],
      { duration: ms(MOTION.ripple_duration), easing: MOTION.easing_ripple, fill: 'forwards' },
    );
    // The circle is Google's pressed ::after: it fades over state_pressed_out_duration once the
    // pointer is up and the press has lasted ripple_minimum_press.
    const pressedAt = performance.now();
    let released = false;
    let fading = false;
    const fade = () => {
      if (fading) return;
      fading = true;
      ripple.animate([{ opacity: getComputedStyle(ripple).opacity }, { opacity: 0 }], { duration: ms(MOTION.state_pressed_out_duration), easing: 'linear', fill: 'forwards' }).onfinish = () => ripple.remove();
    };
    const release = () => {
      if (!released) return;
      const wait = ms(MOTION.ripple_minimum_press) - (performance.now() - pressedAt);
      if (wait > 0) window.setTimeout(fade, wait);
      else fade();
    };
    const onUp = () => {
      released = true;
      release();
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
    };
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);

  };
  document.addEventListener('pointerdown', onDown);
  return () => document.removeEventListener('pointerdown', onDown);
}

export interface TooltipState {
  text: string;
  /** Centre x and bottom y of the hovered button, viewport px. */
  x: number;
  y: number;
}

/** Text of the tooltip attached to a button: its `data-tooltip` attribute (docs/99, 2026-09-15:
 *  Google's hidden `[role=tooltip]` nodes were dropped; nothing invisible stays in the DOM). */
function tooltipFor(target: EventTarget | null): { host: HTMLElement; text: string } | null {
  if (!(target instanceof Element)) return null;
  const host = target.closest('[data-tooltip]');
  if (!(host instanceof HTMLElement)) return null;
  const text = host.dataset.tooltip?.trim();
  return text ? { host, text } : null;
}

/**
 * Google shows a button's tooltip `tooltip_delay` after the pointer arrives and hides it when the
 * pointer leaves or the button is pressed. `onChange` receives the tooltip to show, or null.
 */
export function installTooltips(onChange: (t: TooltipState | null) => void): () => void {
  let timer = 0;
  let current: HTMLElement | null = null;
  const hide = () => {
    window.clearTimeout(timer);
    if (current) {
      current = null;
      onChange(null);
    }
  };
  const onOver = (e: MouseEvent) => {
    const found = tooltipFor(e.target);
    if (!found) {
      hide();
      return;
    }
    if (found.host === current) return;
    window.clearTimeout(timer);
    if (current) {
      current = null;
      onChange(null);
    }
    timer = window.setTimeout(() => {
      const r = found.host.getBoundingClientRect();
      current = found.host;
      onChange({ text: found.text, x: r.left + r.width / 2, y: r.bottom + layoutNumber(LAYOUT.tooltip_gap) });
    }, ms(MOTION.tooltip_delay));
  };
  const onOut = (e: MouseEvent) => {
    const to = e.relatedTarget;
    if (to instanceof Element && current && current.contains(to)) return;
    const found = tooltipFor(to);
    if (!found || found.host !== current) hide();
  };
  document.addEventListener('mouseover', onOver);
  document.addEventListener('mouseout', onOut);
  document.addEventListener('pointerdown', hide, true);
  window.addEventListener('blur', hide);
  return () => {
    document.removeEventListener('mouseover', onOver);
    document.removeEventListener('mouseout', onOut);
    document.removeEventListener('pointerdown', hide, true);
    window.removeEventListener('blur', hide);
    window.clearTimeout(timer);
  };
}
