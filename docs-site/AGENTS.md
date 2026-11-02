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
- `src/content/docs/*.md` and `src/content.config.ts`: the docs collection. `src/lib/docs.ts` builds nav, pager links, Markdown exports and the llms files; `src/lib/markdown/` holds the hast plugins (code frames, callouts, heading anchors, tables).
- `src/layouts/BaseLayout.astro` and `src/pages/` are the only `.astro` files; components are `.tsx` in `src/components/` with colocated tests.
- `scripts/build-demo.mjs` builds the demo; the iframe talks to the page through the mock-only bridge in `../src/core/mock/embed-bridge.ts`.
- `tests/docs-pages.test.ts` builds the site and checks every docs page, link, anchor and Markdown export.

## Analytics

`src/lib/analytics.ts` is the only module that imports `posthog-js`. It reads `PUBLIC_POSTHOG_KEY` and `PUBLIC_POSTHOG_HOST` (default US cloud). With an empty key, the normal state in dev, tests and CI, the SDK is never loaded and `track` is a silent no-op. Calls made before the SDK loads are buffered (up to 50) and replayed in order.

Events are declared in `src/lib/events.ts`; `track()` accepts only those names. Use snake_case `object_verb` in the past tense (`download_clicked`) with snake_case properties. Never send free text users typed, emails or tokens. Config: memory persistence (no cookie banner needed), autocapture and session recording off. The docs privacy page describes this; keep it in step.

## Build workarounds

- TypeScript is pinned to 6.x because `astro check` does not support TypeScript 7.

## Deployment

`.github/workflows/site.yml` deploys to the Cloudflare Pages project `statglass`
(https://statglass.steamfield.io, a proxied CNAME to `statglass.pages.dev` in the
`steamfield.io` zone) on every push to main that touches `docs-site/` or the app
sources the site renders. It runs `make install lint check test build` on Linux,
then `wrangler pages deploy dist`.

## Pending user actions

- Add the `CLOUDFLARE_API_TOKEN` repository secret (Account > Cloudflare Pages > Edit); the deploy step fails with that message until it exists.
- Create a PostHog project and set the `PUBLIC_POSTHOG_KEY` repository variable.
