# statglass-site

The StatGlass landing page and documentation site: static Astro with React components, Tailwind and Markdown docs. It lives in the StatGlass repo but has its own toolchain (Prettier, not the root Biome config) and its own `package.json`. Run every command from `docs-site/`. The demo build also needs the root's `node_modules`; `make install` installs both.

## Commands

| Command         | Purpose                                                                            |
| --------------- | ---------------------------------------------------------------------------------- |
| `make install`  | Install root and site dependencies (frozen lockfile when `CI` is set)              |
| `make demo`     | Build the app with its mock transport into `public/demo/`; cached by an input hash |
| `make dev`      | Dev server on `PORT_WEB` (default 4321); builds the demo first                     |
| `make lint`     | Fail if Prettier would change any file                                             |
| `make format`   | Format with Prettier                                                               |
| `make check`    | `astro check`: types and content                                                   |
| `make test`     | Vitest unit, component and build tests                                             |
| `make coverage` | Tests with coverage                                                                |
| `make build`    | Static build into `dist/` (builds the demo first)                                  |
| `make preview`  | Serve the production build                                                         |
| `make clean`    | Remove `dist/`, `.astro/`, `public/demo/` and the demo stamp                       |

`stacks verify` runs install, lint, check, test, build, a README check, and probes `make dev`. Don't edit `.stacks/manifest.json`.

## The app boundary

Site code may import the app only through two aliases in `astro.config.mjs`: `@statglass/widgets` (`src/app/widgets/index.ts`, render-only components and their prop types) and `@statglass/tokens.css` (the app's design tokens). Never import `~/`, `@core/` or `@tauri-apps/*` from site code; those aliases exist only because the widgets use them internally. Colors, weights (400/510/590), radii and motion come from the app tokens; see `../plan/design-system.md`.

## Layout

- `src/config.ts`: site title, description, repo and nav.
- `src/content/docs/*.md` and `src/content.config.ts`: the docs collection. `src/lib/docs.ts` configures `@steamfield/ui/docs` for nav, pager links, Markdown exports and the llms files; the hast plugins (code frames, callouts, heading anchors, tables) come from `@steamfield/ui/markdown`, and `src/styles/sf-theme.css` maps the site tokens onto the library's styles.
- `src/layouts/BaseLayout.astro` and `src/pages/` are the only `.astro` files; components are `.tsx` in `src/components/` with colocated tests.
- `scripts/build-demo.mjs` builds the demo; the iframe talks to the page through the mock-only bridge in `../src/core/mock/embed-bridge.ts`.
- `tests/docs-pages.test.ts` builds the site and checks every docs page, link, anchor and Markdown export.

## Analytics

`src/lib/analytics.ts` is the only module that imports `posthog-js`. It never reads env: a module script in `BaseLayout.astro` passes `PUBLIC_POSTHOG_KEY` and `PUBLIC_POSTHOG_HOST` (default US cloud) to `initAnalytics`. With an empty key, the normal state in dev, tests and CI, the SDK is never loaded and `track` is a silent no-op. Calls made before the SDK loads are buffered (up to 50) and replayed in order.

Events are declared in `src/lib/events.ts`; `track()` accepts only those names. Use snake_case `object_verb` in the past tense (`download_clicked`) with snake_case properties. Never send free text users typed, emails or tokens. All five Steamfield sites send to one shared Steamfield PostHog project; every event carries a `site` property (stamped in `before_send`, since opting in resets super properties), `"statglass"` here. Consent: PostHog runs cookieless (`cookieless_mode: "on_reject"`, opted out by default) until the visitor accepts, then uses `localStorage+cookie` across subdomains; the decision lives in the `sf-consent` cookie on `.steamfield.io`. When the build has a key, `BaseLayout.astro` creates the store with `createConsentStore({ domain: "steamfield.io" })` from `@steamfield/ui/consent`, passes it to `initAnalytics` and mounts the banner with `mountConsentBanner`, linking the shared privacy page at https://steamfield.io/privacy (also in the footer and on the docs privacy page); the banner's `--sf-consent-*` tokens are mapped in `src/styles/sf-theme.css`. The consent store is the source of truth: pass it to `initAnalytics` as `consent`, and PostHog follows it (granted opts in; undecided or declined goes back to cookieless and stores nothing). The layout only calls `initAnalytics` when the build has a key, so a keyless build ships no analytics code. The docs privacy page describes analytics; keep it in step.

What is captured (the module is copied from stacks-api and shared by every site; its comments explain each option):

- Every visitor, cookieless included: pageviews and page leaves, autocapture (clicks, rage clicks, field changes, form submits; never input values) and web vitals. Page URLs go out without their hash. Each page loads PostHog's remote config script and web vitals library from PostHog's CDN; feature flags stay off, so `/flags` is never requested.
- After Accept only: heatmaps and session recording with console logs. Recordings mask every input and the text of anything marked `data-ph-mask`, and never record network bodies or headers. The site has no text field of its own, so nothing here needs `data-ph-mask`. The demo `<iframe>`s (`src/islands/PopoverDemo.tsx`, `DashboardDemo.tsx`) carry `ph-no-capture`, which blocks them from replays: the real app inside shows typed filter text as plain table text. `src/islands/demo-frames.test.tsx` checks the class. `before_send` drops heatmap, dead-click and snapshot events unless PostHog's consent record is granted, and a page restored from the back-forward cache re-checks it. PostHog Logs' console capture is kept out (`__extensionClasses: { logs: undefined }`).
- Autocapture reads an element's text before consent, and `data-ph-mask` doesn't apply to it: never echo typed text in an `a`, `button`, `label` or `form`.
- https://steamfield.io/privacy lists all of this; keep the site's own copy in step. Tests: `src/lib/analytics.test.ts` runs the real posthog-js with remote config from `tests/fixtures/posthog/` (its README says how to refresh it), including a config with everything switched on. There is no Playwright here, so masking and network capture in real recordings are checked by the Chromium specs in stacks-api, rr and decks.

## Build workarounds

- TypeScript is pinned to 6.x because `astro check` does not support TypeScript 7.

## Deployment

Cloudflare Pages builds and serves the site: project `statglass`, connected to
the GitHub repository, at https://statglass.steamfield.io (a proxied CNAME to
`statglass.pages.dev` in the `steamfield.io` zone). Pushes to main deploy to
production and other branches get preview URLs, but only when a push touches
`docs-site/` or the app sources the site renders (`src/`, `index.html`,
`vite.config.ts`, `tsconfig*.json`, `package.json`, `bun.lock`). The build runs
from `docs-site/` with `BUN_VERSION` and `NODE_VERSION` set in the project:
root and site `bun install --frozen-lockfile`, `bun scripts/build-demo.mjs`,
`bunx astro build`, output `dist`. The build settings live in the Cloudflare
dashboard, not in this repository; keep them in step with the `build` target.

## Pending user actions

- In the shared Steamfield PostHog project, set `PUBLIC_POSTHOG_KEY` (and `PUBLIC_POSTHOG_HOST` if not US cloud) as environment variables for the Production environment only on the `statglass` Pages project, and enable "Cookieless server hash mode" and "Discard client IP data" in its settings.
