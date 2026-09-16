# Bundled fonts

| File | Source | License |
|---|---|---|
| `GoogleSansFlex-VF.ttf` | google/fonts `ofl/googlesansflex/GoogleSansFlex[GRAD,ROND,opsz,slnt,wdth,wght].ttf` | `OFL.txt` (SIL OFL 1.1); trademarks in `TRADEMARKS-GoogleSansFlex.md` |
| `Roboto-VF.ttf`, `Roboto-Italic-VF.ttf` | google/fonts `ofl/roboto/Roboto[wdth,wght].ttf`, `Roboto-Italic[wdth,wght].ttf` | `OFL-Roboto.txt` (SIL OFL 1.1) |
| `MaterialSymbolsOutlined-subset.woff2` | google/material-design-icons `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf`, subset to the names of `docs/design/m3/icons.txt` (axes kept) | `LICENSE-MaterialSymbols.txt` (Apache 2.0) |

The three families are the Material 3 typefaces of the app (`docs/11-material3.md` section 4):
Roboto is `--md-ref-typeface-plain`, Google Sans Flex is `--md-ref-typeface-brand`, Material
Symbols Outlined is `--md-icon-font`. Nothing is loaded from the network.

Regenerate the icon subset whenever `docs/design/m3/icons.txt` changes (`docs/11` section 4):

```bash
curl -L -o /tmp/claude-1000/MaterialSymbolsOutlined.ttf \
  'https://github.com/google/material-design-icons/raw/master/variablefont/MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf'
python3 -m fontTools.subset /tmp/claude-1000/MaterialSymbolsOutlined.ttf \
  --glyphs="$(paste -sd, docs/design/m3/icons.txt)" \
  --text="$(tr -d '\n' < docs/design/m3/icons.txt | tr -s '_a-z' | fold -w1 | sort -u | paste -sd '')" \
  --layout-features='rlig,liga' --no-layout-closure --flavor=woff2 \
  --output-file=src-tauri/resources/fonts/MaterialSymbolsOutlined-subset.woff2
```

`icons.txt` lists glyph names, which are the canonical ligature names of Material Symbols
(`call`, `place`, `open_in_new`; the aliases `phone`, `location_on`, `launch` are not glyph names
and `pyftsubset` rejects them). `<md-icon>place</md-icon>` renders the icon. The webview loads
these files through the `public/fonts` symlink (vite copies them into `dist/`).

Until M5-T1 of the Material 3 transition `GoogleMaterialIcons-subset.woff2` (Material Icons,
Apache 2.0, the icon family of the previous Google-replica UI) stays for the unmigrated components.
