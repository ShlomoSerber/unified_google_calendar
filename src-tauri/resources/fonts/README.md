# Bundled fonts

| File | Source | License |
|---|---|---|
| `GoogleSansFlex-VF.ttf` | google/fonts `ofl/googlesansflex/GoogleSansFlex[GRAD,ROND,opsz,slnt,wdth,wght].ttf` | `OFL.txt` (SIL OFL 1.1); trademarks in `TRADEMARKS-GoogleSansFlex.md` |
| `Roboto-VF.ttf`, `Roboto-Italic-VF.ttf` | google/fonts `ofl/roboto/Roboto[wdth,wght].ttf`, `Roboto-Italic[wdth,wght].ttf` | `OFL-Roboto.txt` (SIL OFL 1.1) |
| `GoogleMaterialIcons-subset.woff2` | `fonts.gstatic.com/s/googlematerialicons/v144/…otf` (the icon family calendar.google.com uses, measured in `docs/design/measurements/gm3-light.json`), subset to `docs/design/icons.txt` | `LICENSE-MaterialSymbols.txt` (Apache 2.0, Material Icons) |

`Google Sans Text` and `Google Sans` are proprietary and are not bundled: `src/styles/fonts.css`
loads them at runtime from `fonts.googleapis.com` with `Google Sans Flex` as fallback
(docs/04-fidelidad-visual.md section 6, docs/99-decisiones.md).

Regenerate the icon subset:

```bash
python3 -m fontTools.subset GoogleMaterialIcons.otf --glyphs="$(paste -sd, docs/design/icons.txt)" \
  --text="$(letters used by those names)" --layout-features='rlig,liga' --no-layout-closure \
  --flavor=woff2 --output-file=GoogleMaterialIcons-subset.woff2
```

Ligatures use the icon names, so `<i class="icon">location_on</i>` renders the icon. The webview
loads these files through the `public/fonts` symlink (vite copies them into `dist/`).
