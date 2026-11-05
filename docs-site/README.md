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

PostHog is wired up but stays off until `PUBLIC_POSTHOG_KEY` is set at build time (and optionally `PUBLIC_POSTHOG_HOST`, which defaults to US cloud). `BaseLayout.astro` reads both and passes them to `initAnalytics` in `src/lib/analytics.ts`; with no key the SDK is never loaded. With a key, every visitor is counted without cookies (pageviews, clicks, web vitals), and heatmaps and masked session recordings run only after the visitor accepts the consent banner; see `AGENTS.md` and https://steamfield.io/privacy.

## License

Copyright (c) 2026 Riley Hilliard

## Writing blog posts

Add an MDX file under `src/content/blog/`. The file name becomes its `/blog/<id>`
URL. Required frontmatter: `title` (1–48 characters), `description` (120–160),
`published` (date), `keyword`, `feature`, and `relatedDocs` (docs collection ids).
Optional `headline` supplies a longer h1 (up to 90 characters); `updated` must be
on or after publication. See `AGENTS.md` for feature values and all demo props.

```mdx
---
title: Exploring GPU readings
description: Explore GPU readings in StatGlass with an interactive dashboard demo, a compact simulated card and links to the metric reference for your Mac.
published: 2026-11-04
keyword: check GPU usage on Mac
feature: gpu
relatedDocs: [metrics]
---

import { ArticleDemo } from "@/islands/ArticleDemo";

## Read the GPU chart

<ArticleDemo
  client:visible
  route="/dashboard/gpu"
  title="StatGlass GPU dashboard"
  caption="The GPU page on simulated data."
  fallback="gpu"
/>
```

Import each demo directly from its module and use `client:visible`: `ArticleDemo`,
`ModuleLiveCard`, `PopoverFigure`, `MenuBarStyles` and `AgentSession`. Dashboard
figures expand beyond the reading column; narrow screens get a fallback when
provided and a link to open the demo. Every module card is labeled as simulated.
Code fence titles, callouts and tables use the same processor as docs. The frame
provides h1, dates, reading time, related links and install action; use h2/h3 in
post bodies. Posts are included in RSS (`/blog/rss.xml`), the sitemap and
`/llms.txt`.
