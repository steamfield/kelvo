#!/usr/bin/env node
// Exports the brand pack in brand/exports/ and the site's icons and social
// card in docs-site/public/ from the SVG sources beside this file.
//
//   node brand/build.mjs            raster exports only
//   node brand/build.mjs --og URL   also screenshot the landing page at URL
//
// Needs rsvg-convert and cwebp (brew install librsvg webp). The OG screenshot
// uses the root Playwright install; serve a production build first
// (cd docs-site && make build && make preview).
import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const brand = join(root, "brand");
const exportsDir = join(brand, "exports");
const site = join(root, "docs-site/public");
const tmp = mkdtempSync(join(tmpdir(), "statglass-brand-"));

mkdirSync(exportsDir, { recursive: true });

/** Rasterize an SVG at a pixel width and encode it as lossless WebP. */
function webp(svg, width, out) {
  const png = join(tmp, "frame.png");
  execFileSync("rsvg-convert", [
    "--width",
    String(width),
    "--keep-aspect-ratio",
    "-o",
    png,
    svg,
  ]);
  execFileSync("cwebp", ["-quiet", "-lossless", "-exact", png, "-o", out]);
}

// iOS and PWA launchers apply their own mask, so the touch icon is the app
// icon's squircle content cropped to a full-bleed square.
const appIcon = readFileSync(join(brand, "statglass-app-icon.svg"), "utf8");
const fullBleed = join(tmp, "app-icon-full-bleed.svg");
writeFileSync(
  fullBleed,
  appIcon
    .replace('viewBox="0 0 1024 1024"', 'viewBox="100 100 824 824"')
    .replace(/ rx="18[35]"/g, "")
);

const jobs = [
  ...["light", "dark"].flatMap((theme) => {
    const suffix = theme === "dark" ? "-dark" : "";
    return [
      ...[512, 1024, 2048].map((w) => [
        join(brand, `statglass-logo${suffix}.svg`),
        w,
        join(exportsDir, `statglass-logo-${theme}-${w}w.webp`),
      ]),
      ...[64, 256, 512, 1024].map((w) => [
        join(brand, `statglass-icon${suffix}.svg`),
        w,
        join(exportsDir, `statglass-icon-${theme}-${w}.webp`),
      ]),
    ];
  }),
  ...[256, 512, 1024].map((w) => [
    join(brand, "statglass-app-icon.svg"),
    w,
    join(exportsDir, `statglass-app-icon-${w}.webp`),
  ]),
  [fullBleed, 180, join(site, "apple-touch-icon.webp")],
  [fullBleed, 192, join(site, "icon-192.webp")],
  [fullBleed, 512, join(site, "icon-512.webp")],
];

for (const [svg, width, out] of jobs) webp(svg, width, out);

const ogIndex = process.argv.indexOf("--og");
if (ogIndex !== -1) {
  const url = process.argv[ogIndex + 1];
  if (!url) throw new Error("--og needs the landing page URL");
  const { chromium } = await import(
    join(root, "node_modules/playwright/index.mjs")
  );
  const browser = await chromium.launch();
  // 1280x670 holds the hero and the whole popover at the 1.91:1 OG ratio;
  // 2x gives link previews (iMessage, Slack) a 2560x1340 card.
  const page = await browser.newPage({
    viewport: { width: 1280, height: 670 },
    deviceScaleFactor: 2,
    colorScheme: "dark",
    reducedMotion: "reduce",
  });
  await page.goto(url, { waitUntil: "networkidle" });
  // Let the embedded popover fill its sparklines from the mock transport.
  await page.waitForTimeout(2500);
  const png = join(tmp, "og.png");
  await page.screenshot({ path: png });
  await browser.close();
  for (const out of [
    join(site, "og.webp"),
    join(exportsDir, "statglass-og.webp"),
  ]) {
    execFileSync("cwebp", ["-quiet", "-q", "90", "-m", "6", png, "-o", out]);
  }
}

rmSync(tmp, { recursive: true });
