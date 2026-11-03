# StatGlass brand assets

The SVGs here are the sources. Everything in `exports/` and the site's icons and
social card are generated from them by `build.mjs`; don't edit those by hand.

| Source                     | Use                                                   |
| -------------------------- | ----------------------------------------------------- |
| `statglass-logo.svg`       | Wordmark on light backgrounds                         |
| `statglass-logo-dark.svg`  | Wordmark on dark backgrounds                          |
| `statglass-icon.svg`       | Mark on light backgrounds (also the site favicon)     |
| `statglass-icon-dark.svg`  | Mark on dark backgrounds                              |
| `statglass-app-icon.svg`   | macOS app icon: the mark on its dark squircle         |

Raster exports are WebP. Logos and icons are lossless with transparency:

- `statglass-logo-{light,dark}-{512,1024,2048}w.webp` (width in pixels)
- `statglass-icon-{light,dark}-{64,256,512,1024}.webp`
- `statglass-app-icon-{256,512,1024}.webp`
- `statglass-og.webp`: the 2560x1340 social card, a 2x screenshot of the landing hero in dark mode

The build also writes `docs-site/public/og.webp` (same card), `apple-touch-icon.webp`
(180) and `icon-{192,512}.webp` for the web manifest. Those three are the app icon cropped
to a full-bleed square, since iOS and launchers apply their own mask.

## Regenerating

Needs `rsvg-convert` and `cwebp` (`brew install librsvg webp`).

```bash
node brand/build.mjs
```

The social card is a screenshot, so retake it whenever the landing hero changes. Serve a
production build of the site, then pass its URL:

```bash
cd docs-site && make build && make preview
node brand/build.mjs --og http://localhost:4321/
```
