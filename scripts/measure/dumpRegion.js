// Paste this whole file into the Chrome DevTools console on calendar.google.com
// (or into the app's inspector), then run: dumpRegion($0)
// It downloads a JSON with the bounding rect and the non-default computed styles
// of every visible descendant of the selected node. See docs/04-fidelidad-visual.md.
(() => {
  const PROPS = ['display','position','fontFamily','fontSize','fontWeight','lineHeight',
    'letterSpacing','color','backgroundColor','borderTop','borderRight','borderBottom',
    'borderLeft','borderRadius','paddingTop','paddingRight','paddingBottom','paddingLeft',
    'marginTop','marginRight','marginBottom','marginLeft','boxShadow','opacity','width',
    'height','minWidth','minHeight','gap','flexDirection','alignItems','justifyContent',
    'textTransform','whiteSpace','overflow','zIndex','fill','outline','cursor'];
  const DEFAULTS = (() => {
    const d = document.createElement('div'); document.body.appendChild(d);
    const cs = getComputedStyle(d); const o = {}; PROPS.forEach(p => o[p] = cs[p]);
    d.remove(); return o; })();
  window.dumpRegion = function (root = $0, { maxDepth = 60, keepDefaults = false, hidden = false } = {}) {
    if (typeof root === 'string') root = document.querySelector(root);
    const rootRect = root.getBoundingClientRect();
    const out = { url: location.href, dpr: devicePixelRatio, viewport: [innerWidth, innerHeight],
      theme: document.documentElement.getAttribute('data-theme') || null,
      colorScheme: getComputedStyle(document.documentElement).colorScheme,
      rootRect: [rootRect.x, rootRect.y, rootRect.width, rootRect.height], nodes: [] };
    const walk = (el, path, depth) => {
      if (depth > maxDepth) return;
      const cs = getComputedStyle(el); const r = el.getBoundingClientRect();
      if (!hidden && (cs.display === 'none' || cs.visibility === 'hidden' || (r.width === 0 && r.height === 0))) return;
      const style = {};
      for (const p of PROPS) if (keepDefaults || cs[p] !== DEFAULTS[p]) style[p] = cs[p];
      const text = [...el.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent.trim()).filter(Boolean).join(' ');
      out.nodes.push({ path, tag: el.tagName.toLowerCase(), cls: el.className && String(el.className).trim(),
        role: el.getAttribute('role'), aria: el.getAttribute('aria-label'), text: text.slice(0, 80) || undefined,
        rect: [r.x - rootRect.x, r.y - rootRect.y, r.width, r.height].map(v => +v.toFixed(2)),
        abs: [r.x, r.y].map(v => +v.toFixed(2)), style });
      let i = 0; for (const c of el.children) walk(c, `${path}/${c.tagName.toLowerCase()}[${i++}]`, depth + 1);
    };
    walk(root, root.tagName.toLowerCase(), 0);
    const json = JSON.stringify(out, null, 1);
    try { copy(json); console.log('JSON copied to clipboard'); } catch {}
    const a = Object.assign(document.createElement('a'), { href: URL.createObjectURL(new Blob([json], { type: 'application/json' })),
      download: `gcal-${(root.getAttribute('aria-label') || root.tagName).replace(/\W+/g, '_')}-${Date.now()}.json` });
    a.click(); console.log(`${out.nodes.length} nodes`); return out;
  };
  console.log('Usage: dumpRegion($0) | dumpRegion("[role=main]") | dumpRegion(el,{keepDefaults:true})');
})();
