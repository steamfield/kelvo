# statglass-site

The StatGlass landing page and documentation site.

A static [Astro](https://astro.build) site. Its demos are the real app: widgets render from `src/app/widgets` through `@statglass/widgets`, and the live demo is the app itself, built with its mock transport into `public/demo/`. When the app changes, the site does too.

## Quickstart

Requires [bun](https://bun.sh) and make. The demo build uses the app's dependencies, so `make install` installs the repo root as well.

```bash
make install
make dev        # http://localhost:4321
```

## Writing docs

Docs pages are Markdown files in `src/content/docs/`, validated at build time against the schema in `src/content.config.ts` (title, description of at most 160 characters, group, order). Each page is also published as Markdown (`/docs/install.md`), and `/llms.txt` and `/llms-full.txt` list them for agents. Code fences take `title="..."`, and `> [!NOTE]` style blockquotes become callouts. Link between pages as `/docs/<id>`, without a trailing slash; the build test fails on links that don't resolve.

## Commands

| Command         | What it does                                                   |
| --------------- | -------------------------------------------------------------- |
| `make dev`      | Dev server (builds the demo first)                             |
| `make demo`     | Build the app demo into `public/demo/`; skipped when unchanged |
| `make lint`     | Check formatting                                               |
| `make check`    | Type-check and validate content                                |
| `make test`     | Unit, component and build tests                                |
| `make coverage` | Tests with coverage                                            |
| `make build`    | Production build into `dist/`                                  |
| `make preview`  | Serve the production build                                     |
| `make clean`    | Remove `dist/`, the built demo and caches                      |

## Analytics

PostHog is wired up but stays off until `PUBLIC_POSTHOG_KEY` is set at build time (and optionally `PUBLIC_POSTHOG_HOST`, which defaults to US cloud). With no key the SDK is never loaded. See `src/lib/analytics.ts`.

## License

Copyright (c) 2026 Riley Hilliard
