// Injected into calendar.google.com by scripts/measure/animations.mjs. Records, frame by frame,
// every Web Animation the page runs (CSS transitions, CSS animations, element.animate) plus the
// box, opacity and transform of a few watched elements, so JS-driven (FLIP) motion that never
// touches the animations API is caught too. Everything is read; nothing is written to the page.
window.__animRec = (() => {
  const MAX_FRAMES = 120; // 2 s at 60 fps
  const rect = (el) => {
    const r = el.getBoundingClientRect();
    return [+r.x.toFixed(2), +r.y.toFixed(2), +r.width.toFixed(2), +r.height.toFixed(2)];
  };
  const cls = (el) => (typeof el.className === 'string' ? el.className : el.getAttribute('class') || '');
  const desc = (el) => {
    if (!el || !(el instanceof Element)) return null;
    return {
      tag: el.tagName,
      cls: cls(el),
      role: el.getAttribute('role'),
      label: el.getAttribute('aria-label'),
      text: (el.innerText || '').trim().slice(0, 40),
      rect: rect(el),
      parentCls: el.parentElement ? cls(el.parentElement) : null,
    };
  };
  const keyframes = (effect) => {
    try {
      return effect.getKeyframes().map((k) => {
        const out = {};
        for (const [p, v] of Object.entries(k)) if (p !== 'composite' && p !== 'computedOffset') out[p] = v;
        return out;
      });
    } catch {
      return null;
    }
  };
  const timing = (effect) => {
    const t = effect.getComputedTiming();
    return { delay: t.delay, duration: t.duration, easing: t.easing, fill: t.fill, iterations: t.iterations, endTime: t.endTime };
  };
  const style = (el) => {
    const cs = getComputedStyle(el);
    return { opacity: cs.opacity, transform: cs.transform, visibility: cs.visibility, display: cs.display };
  };

  let state = null;

  function sample() {
    if (!state) return;
    const now = performance.now() - state.t0;
    for (const a of document.getAnimations()) {
      if (state.seen.has(a)) continue;
      state.seen.add(a);
      const effect = a.effect;
      const target = effect && effect.target;
      state.found.push({
        at: +now.toFixed(1),
        type: a.constructor.name,
        name: a.animationName || a.transitionProperty || a.id || null,
        pseudo: (effect && effect.pseudoElement) || null,
        currentTime: a.currentTime,
        playState: a.playState,
        target: desc(target),
        timing: effect ? timing(effect) : null,
        keyframes: effect ? keyframes(effect) : null,
      });
    }
    if (state.frames.length < MAX_FRAMES) {
      const row = { at: +now.toFixed(1), watch: {} };
      let changed = state.frames.length === 0;
      for (const [key, sel] of Object.entries(state.watch)) {
        let els;
        try { els = [...document.querySelectorAll(sel)].filter((e) => e.getBoundingClientRect().width > 0 || e.getBoundingClientRect().height > 0).slice(0, 4); } catch { els = []; }
        row.watch[key] = els.map((e) => ({ cls: cls(e), rect: rect(e), ...style(e) }));
        const prev = state.last[key];
        const cur = JSON.stringify(row.watch[key]);
        if (prev !== cur) changed = true;
        state.last[key] = cur;
      }
      if (changed) state.frames.push(row);
    }
    state.raf = requestAnimationFrame(sample);
  }

  return {
    /** Start recording. `watch` maps a key to a CSS selector whose matches are sampled every frame. */
    start(watch) {
      this.stop();
      state = { t0: performance.now(), seen: new Set(), found: [], frames: [], watch: watch || {}, last: {}, raf: 0 };
      sample();
    },
    /** Stop and return what was recorded. */
    stop() {
      if (!state) return null;
      cancelAnimationFrame(state.raf);
      const out = { found: state.found, frames: state.frames, elapsed: +(performance.now() - state.t0).toFixed(1) };
      state = null;
      return out;
    },
    /**
     * Computed style of `el` and its descendants for the properties hover states change, keyed by
     * a path of class names, so two snapshots (rest and hover) can be diffed.
     */
    styles(el, limit) {
      const props = ['background-color', 'background-image', 'color', 'fill', 'box-shadow', 'opacity', 'transform', 'border-color', 'outline-color', 'outline-width', 'visibility', 'text-decoration-line', 'transition-property', 'transition-duration', 'transition-timing-function', 'transition-delay', 'animation-name', 'animation-duration'];
      const out = {};
      const all = [el, ...el.querySelectorAll('*')].slice(0, limit || 80);
      all.forEach((e, i) => {
        const cs = getComputedStyle(e);
        const entry = { tag: e.tagName, cls: cls(e), rect: rect(e) };
        for (const p of props) entry[p] = cs.getPropertyValue(p);
        for (const pseudo of ['::before', '::after']) {
          const ps = getComputedStyle(e, pseudo);
          if (ps.content && ps.content !== 'none' && ps.content !== 'normal') {
            const pe = {};
            for (const p of props) pe[p] = ps.getPropertyValue(p);
            pe.content = ps.content;
            entry[pseudo] = pe;
          }
        }
        out[`${i}:${e.tagName}`] = entry; // keyed by position: hover adds classes, the key must not move
      });
      return out;
    },
  };
})();
