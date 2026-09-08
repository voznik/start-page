# Self-hosted font asset

CLAUDE.md / T0.2 step 4 forbids the CDN `<link>` every Ratzilla example uses. This directory ships
a self-hosted, subsetted Nerd Font instead.

## Font

- **Family**: JetBrainsMono Nerd Font Mono, Regular
- **Nerd Fonts patch version**: v3.5.1 (`ryanoasis/nerd-fonts` release tag)
- **Upstream base font**: JetBrains Mono (patched by the Nerd Fonts project with box-drawing,
  powerline, and icon glyphs)
- **License**: SIL Open Font License 1.1 (`https://scripts.sil.org/OFL`) — confirmed from the
  font's own `name` table (ID 13/14), copyright "2020 The JetBrains Mono Project Authors"
- **Source**: `https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/JetBrainsMono.zip`,
  file `JetBrainsMonoNerdFontMono-Regular.ttf` inside the archive

A single patched font was chosen over a base font + symbols-only fallback font: one `@font-face`
covers both Latin text and Nerd Font icon glyphs, so there's no fallback-chain ordering to get
wrong.

## Subsetting — two variants

The unpatched font is 12,226 codepoints / 2.57 MB TTF (every script JetBrains Mono covers, plus
every Nerd Font icon family including ones this project doesn't render). Two subsets exist:

### `-subset-min.woff2` — the one `font.css` actually uses

Latin-1 text, box drawing (U+2500–U+257F), block elements (U+2580–U+259F), Powerline
(U+E0A0–U+E0D4), and Font Awesome only (U+F000–U+F2E0). This is the aggressive variant: T0.2
measures first-paint against the shipped bundle, and a several-hundred-KB font would dominate and
distort that number, so only the icon family phase 0 actually needs (Font Awesome — the required
glyphs F07B/F09B/F015 are all Font Awesome) is kept.

```bash
python -m fontTools.subset JetBrainsMonoNerdFontMono-Regular.ttf \
  --output-file=subset-aggressive.ttf \
  --unicodes="U+0020-007E,U+00A0-00FF,U+2500-259F,U+E0A0-E0D4,U+F000-F2E0" \
  --layout-features='*' --glyph-names --symbol-cmap --legacy-cmap \
  --notdef-glyph --notdef-outline --recommended-glyphs \
  --name-IDs='*' --name-legacy --name-languages='*' --no-hinting
woff2_compress subset-aggressive.ttf
mv subset-aggressive.woff2 fonts/JetBrainsMonoNerdFontMono-Regular-subset-min.woff2
```

**116,064 bytes (113.3 KiB)**, 1,126 codepoints.

### `-subset.woff2` — fuller fallback, not wired up

Everything in the min variant plus Devicons, Octicons, Seti-UI, Weather, Codicons, and Pomicons —
useful if a later phase adds icons outside Font Awesome (e.g. a Docker/service-status dashboard
widget commonly wants Devicons or Octicons glyphs).

```bash
python -m fontTools.subset JetBrainsMonoNerdFontMono-Regular.ttf \
  --output-file=subset.ttf \
  --unicodes="U+0020-007E,U+00A0-00FF,U+2010-2027,U+2030-205E,U+2500-259F,U+25A0-25FF,U+E000-E00A,U+E0A0-E0D7,U+E200-E2A9,U+E300-E3EB,U+E5FA-E6B7,U+E700-E7C5,U+EA60-EC1E,U+F000-F2E0,U+F300-F381,U+F400-F532" \
  --layout-features='*' --glyph-names --symbol-cmap --legacy-cmap \
  --notdef-glyph --notdef-outline --recommended-glyphs \
  --name-IDs='*' --name-legacy --name-languages='*' --no-hinting
woff2_compress subset.ttf
mv subset.woff2 fonts/JetBrainsMonoNerdFontMono-Regular-subset.woff2
```

**363,220 bytes (354.7 KiB)**, 2,874 codepoints.

To switch `font.css` to this variant, change the `src: url(...)` in the `@font-face` rule from
`fonts/JetBrainsMonoNerdFontMono-Regular-subset-min.woff2` to
`fonts/JetBrainsMonoNerdFontMono-Regular-subset.woff2` — nothing else changes.

`fonttools` isn't preinstalled (`pip install --user fonttools brotli` — brotli is needed for WOFF2
support in `pyftsubset`). No upstream WOFF2 is published for this asset (Nerd Fonts release only
ships TTF/OTF), so both variants were converted with `woff2_compress` (Google's reference `woff2`
tool, already present on this machine at `/usr/bin/woff2_compress`). `gzip -9` on either woff2
saves almost nothing — WOFF2's own Brotli-based compression already accounts for it — so no
further gzip/brotli step is applied at build time for these files.

| Variant | Bytes | KiB | Codepoints |
|---|---|---|---|
| `-subset-min.woff2` (wired up) | 116,064 | 113.3 | 1,126 |
| `-subset.woff2` (fallback) | 363,220 | 354.7 | 2,874 |
| unpatched source TTF | 2,573,248 | 2,513.9 | 12,226 |

## Glyph coverage verification

Asserted programmatically against each subsetted font's cmap (not inferred from the font name).
Both variants pass — the min variant is the one actually shipped:

```
0x2502 OK uni2502          (box drawing, vertical line)
0x2500 OK uni2500          (box drawing, horizontal line)
0x250c OK uni250C          (box drawing, down-and-right)
0xe0b0 OK pl-left_hard_divider    (powerline)
0xe0b2 OK pl-right_hard_divider   (powerline)
0xf07b OK fa-folder        (Font Awesome icon)
0xf09b OK fa-github        (Font Awesome icon)
0xf015 OK fa-house         (Font Awesome icon)
```
(`total codepoints in subset cmap`: 1,126 for `-min`, 2,874 for the fuller fallback.)

Verified with:

```bash
python3 -c "
from fontTools.ttLib import TTFont
f = TTFont('subset-aggressive.ttf')   # or subset.ttf for the fuller variant
cmap = f.getBestCmap()
required = [0x2502,0x2500,0x250C,0xE0B0,0xE0B2,0xF07B,0xF09B,0xF015]
for cp in required:
    print(hex(cp), 'OK' if cp in cmap else 'MISSING', cmap.get(cp))
print('total codepoints in subset cmap:', len(cmap))
"
```

## `index.html` wiring (for `sp-web-host`)

This crate does not own `index.html` — hand these two lines to whoever does:

```html
<link data-trunk rel="copy-dir" href="assets/fonts" />
<link data-trunk rel="css" href="assets/font.css" />
```

`font.css` sets `pre { font-family: "JetBrainsMono Nerd Font Mono", monospace; ... }` — `pre` is
the only styling hook Ratzilla's `DomBackend` exposes (it renders each terminal cell into `<pre>`
elements). The `@font-face` `src` path is relative to `font.css`'s own location once trunk copies
both into `dist/` — `fonts/JetBrainsMonoNerdFontMono-Regular-subset.woff2` resolves correctly only
if `copy-dir` places `assets/fonts` at `dist/fonts` and `font.css` lands at `dist/font.css`
(trunk's `copy-dir`/`copy-file` flatten to the output root by default). Verify this once
`index.html` exists and `trunk build` runs; adjust the `url()` path if trunk's actual output
layout differs.
