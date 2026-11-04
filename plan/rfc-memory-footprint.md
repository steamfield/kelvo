# RFC: Memory footprint

Status: Phase 1 implemented and measured (see "Phase 1 results"). The GPU process, window
lifecycle and client cache decisions are in D-127 (see "GPU process and lifecycle"); phase 3
is superseded by them. Revised after review (see "Review outcome" at the end).
Date: 2026-11-03, updated 2026-11-04.

## Problem

A user on the dashboard's Processes view, Memory column set, saw StatGlass's app row at 2.7 GB: WebKit
GPU 1.1 GB, the dashboard's WebContent 1.3 GB, the `statglass` process 231 MB, the popover's WebContent
114 MB. The Processes view shows current footprint, so this was a live reading. For a system monitor it
is an embarrassing number, and it is the number a user compares against Stats and iStat Menus.

Measured on 0.5.0 (release build, `/Applications`), 21 minutes after launch, dashboard open:

| Process | Footprint after the spike | Lifetime peak (`ri_lifetime_max_phys_footprint`) |
|---|---:|---:|
| `statglass` | 132 MB | 231 MB |
| WebKit GPU | 50 to 300 MB | 1180 MB |
| WebContent (dashboard) | 211 to 305 MB | 1385 MB |
| WebContent (popover, warm, hidden) | 86 MB | 143 MB |

The lifetime peaks match the screenshot to the megabyte. `statglass top --by memory` over 1-minute
windows puts the 2.7 GB in the single minute at 22:20. The minutes either side measured 0.65 to 1.0 GB,
and the next one 574 MB. With the dashboard open the coalition sat between 350 and 900 MB for the
session. The tray-only benchmark (D-094) measured about 105 MB, and `architecture.md` sets a target of
under 150 MB for the coalition with the warm panel.

That leaves two problems:

1. A transient spike. Something in the dashboard allocates around 2 GB across WebContent (JS heap and
   graphics) and the GPU process, then releases it within a minute. The cause has not been found, and
   it may be the Processes view itself, since that was on screen.
2. With the dashboard open the steady state is 350 to 600 MB, and nothing measures it per route.

`heap` shows `statglass`'s 132 MB includes about 70 MB in 238k live small allocations. Tray-only, the
app process measured 34 MB (D-067). That is live data, not freed pages, and nothing yet says what it is.

## What is already right

The data structures are compact and stay as they are. The live ring is one `Float32Array` per series
(`src/core/live-state.ts`). Process snapshots keep the top 30. The Processes and Containers tables are
virtualized. uPlot instances are destroyed on unmount (`lane-plot.tsx`).
`crates/statglass-engine/tests/memory_gates.rs` gates retained Rust heap. A hidden dashboard's webview
is destroyed after 10 seconds since D-127 (`src-tauri/src/windows.rs`, `DESTROY_AFTER`; 5 minutes when this was written). The Memory route's by-app
table and the CLI already label their figure as the peak.

What's missing: a gate on whole-process footprint, the spike's cause, and the source of the Rust heap
growth.

## Goals

- Find and remove the spike, and add a gate that catches the next one.
- Measure the steady-state footprint of each route and give it a ceiling.
- Have a hidden dashboard cost close to nothing well before five minutes, if measurement shows it
  doesn't already.
- Keep tray-only footprint under the 150 MB coalition target, as it is today.

## Proposal

The phases run in order. Phase 1 decides whether phases 3 and 5 are needed and where phase 2's fix goes.

### Phase 1: measure footprint per scenario, including peaks

`scripts/bench-coalition.sh` (`make bench`) already relaunches the app for each scenario (`stop_app`
then `open -n`). That makes `ri_lifetime_max_phys_footprint`, read from every coalition member at the
end of a run, that scenario's peak since launch. No interval reset or private API is needed. The bench
checks that each member, including the WebKit GPU process, has a new pid after the relaunch, and
discards a run where one doesn't. A peak that includes startup is fine, since startup is part of the
footprint a user sees. For a steady figure, `coalition.rs` samples footprint every 5 s through the
measured window and reports the median as well as the end value.

The bench can only pick a route today (`src-tauri/src/bench.rs`, `route_of` in the script). Phase 1
adds a small interaction driver behind the `bench` feature: a scenario names a sequence (`zoom 30d→1h`
repeated 5 times, `range 1h→24h→1h`, `timeline follow live` on the power lane). The app runs it through
the webview after load, so the run stays hands-off. Timeline scenarios need history, so the bench
identity is seeded first with `seed --dir <bench data dir>`
(`crates/statglass-store/examples/seed.rs`).

The scenarios are tray, popover, each dashboard route once (Processes with Memory sort among them),
Timeline at 30 days, the three interactions above, and "dashboard closed for 60 s". At 150 s each, the
full set takes about 35 minutes. `make bench SCENARIOS=...` runs a subset for day-to-day use.

`perf-budget.json` gains a `footprint` section. It holds the existing 150 MB tray-only coalition target
from `architecture.md` as `target`, plus per-scenario steady and peak ceilings set from measurement
plus headroom as the ratchet baseline. It is advisory until five clean runs exist (the rule
`tray-performance.md` applies to the visible baseline), then it fails `make bench`. Raising a threshold
needs a decision entry (D-060). The gate is local only, consistent with D-118. The Playwright gate
can't stand in for it, because Chromium has no WebKit GPU process and a different memory model.

The results go in this RFC before phase 2 starts.

### Phase 2: remove the spike

The fix follows phase 1's numbers. Review narrowed the candidates to three:

- The Timeline zoom preview (`use-zoom-preview.ts`) clones one `[data-lane-content]` subtree, copying
  its canvases' pixels, and animates it with `scaleX = bounds / target`. Zooming from 30 days to 1 h
  makes that about 720. One copy is tens of MB. The unverified risk is WebKit sizing the animated
  layer's backing store for the scaled extent. A single-bitmap preview with the same transform would
  not fix that. If this is the cause, the fix caps the preview's scale or crossfades instead of
  stretching.
- `lane-plot.tsx` rebuilds its uPlot instance whenever `lo` or `hi` change, and the power lane's
  ceiling comes from the data (`lane-model.ts`). A range switch, or a refresh while following Live,
  can then create a new canvas per lane, while the old ones wait for garbage collection. If this is
  the cause, the fix updates scales on the existing instance with `setScale` and only rebuilds when
  the shape changes.
- The Processes view, which was on screen during the spike. Its scenario in phase 1 runs with Memory
  sort.

### Phase 3: release a hidden dashboard's content early (superseded by D-127)

Not built. A cold dashboard open measured about 110 ms, so D-127 destroys a hidden dashboard
after 10 s instead, which also frees the JS heap that this phase would have kept.

This phase is built only if phase 1's "dashboard closed for 60 s" scenario shows the dashboard's
WebContent and GPU process still well above idle. The post-spike readings (GPU at 50 MB) suggest WebKit
already drops layers for a hidden window.

If it is needed, `hide_dashboard` starts a Rust timer of 30 s that uses the same generation counter as
`DESTROY_AFTER`. Rust owns lifecycle, and JS timers in a hidden webview are throttled. When it fires,
Rust sends a new `ContentReleased` event to the dashboard only. Occlusion and minimise don't trigger it:
`set_visible` handles those for the live channel, and a dashboard covered by another app keeps its
content. On the event, the frontend keeps its shell (sidebar, router, stores, the live ring) and stops
rendering route content, so the DOM, SVG and canvases unmount. Showing the window again re-renders the
current route from the ring. History queries refetch only if React Query has collected them; the client
keeps the default 5-minute `gcTime`, so a reopen inside 5 minutes doesn't refetch. The event is
registered with specta, the bindings are regenerated, and the mock transport gets a counterpart.

A new perf-gate case covers remount after release: it reopens, checks for no long task over
`longTaskMs`, and checks that the CPU 1 h line has no hole. The existing `hiddenResume` case only
covers resuming into a route that is still mounted. Reopen-to-first-paint after release is measured
and reported.

The 5-minute destroy stays as the backstop.

### Phase 4: canvas teardown

Every place that drops a canvas sets its `width` and `height` to 0 first, so WebKit frees the backing
store right away instead of at the next garbage collection. That covers `lane-plot.tsx` (before
`plot.destroy()`, through `plot.ctx.canvas`, because uPlot only removes its root element),
`calendar-heatmap.tsx`, and the zoom preview's copies. It's a few lines per call site. Phase 2's
scenarios show whether it changes the peak.

### Phase 5: find the Rust heap growth

The 70 MB is live, so returning free pages to the OS won't help. On a debug build with
`MallocStackLogging=1`, open the dashboard, use it for a few minutes, close it, then run `heap` and
`malloc_history`. The likely suspects are retained IPC buffers, history answers and process batches.
The fix depends on what that run finds, and it gets recorded here. If the cause is a cache, it gets a
size bound and is checked by the existing `memory_gates.rs` scenario. `malloc_zone_pressure_relief` is
not proposed.

### Phase 6: release the hidden dashboard under memory pressure

A `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE` source, isolated in `src-tauri/src/platform/`, watches warn
and critical. On either, the shell destroys a hidden dashboard immediately instead of waiting for the
timer. The source is pushed by the kernel and costs nothing while idle. It doesn't depend on the memory
module or its 10 s background sampling. Visible windows are left alone, and WebKit already handles
pressure inside WebContent.

## Phase 1 results (2026-11-04)

The first full run used `NO_BUILD=1 SEED=1 SCENARIOS=all make bench`, a bench build of
`perf/memory-footprint-bench`, the M3 Max dev machine on AC, and 30 seeded days. Each
scenario was a fresh launch with a 30 s warm-up and 120 s of measurement. All actions
applied, and no member predated its launch. Footprint is in MB. "Peak" is the sum of each
process's own lifetime peak since launch.

| Scenario | Median | End | Peak | WebContent median / peak | GPU median / peak | `statglass` median / peak |
|---|---:|---:|---:|---:|---:|---:|
| tray | 75 | 75 | 339 | none | 15 / 270 | 46 / 50 |
| popover | 433 | 443 | 458 | 107 / 115 | 266 / 273 | 46 / 51 |
| overview | 584 | 573 | 725 | 237 / 328 | 280 / 301 | 53 / 78 |
| processes | 589 | 620 | 682 | 247 / 323 | 273 / 284 | 51 / 57 |
| cpu | 733 | 704 | 886 | 298 / 403 | 357 / 387 | 51 / 55 |
| gpu | 689 | 689 | 785 | | | |
| memory | 688 | 637 | 801 | | | |
| power | 696 | 712 | 905 | 323 / 473 | 309 / 359 | 50 / 56 |
| network | 577 | 634 | 765 | | | |
| disk | 618 | 670 | 804 | | | |
| battery | 539 | 552 | 660 | | | |
| timeline (15 min) | 657 | 664 | 863 | | | |
| timeline-30d | 623 | 632 | 982 | 250 / 417 | 279 / 362 | 75 / 184 |
| zoom (5x, 30 d) | 643 | 617 | 1087 | 263 / 503 | 290 / 379 | 76 / 187 |
| range (1 h / 24 h, 3x) | 819 | 788 | 1142 | 403 / 625 | 349 / 409 | 65 / 89 |
| follow (Live, 1 h) | 650 | 612 | 884 | | | |
| closed (hidden from ~23 s) | 261 | 266 | 706 | 175 / 308 | 19 / 300 | 54 / 79 |

Blank cells are in the run's JSON but are not repeated here.

What this changes:

- The 2.7 GB spike did not reproduce. The highest peak was 1.14 GB (`range`). The user's
  session had a real 30-day history with more series and processes, unscripted
  interactions, and 21 minutes of uptime. The next repro attempt runs the same scenarios
  against a copy of a real history file instead of seeded data.
- Whenever any window is visible, the WebKit GPU process holds 265 to 410 MB, and that
  includes the popover alone. When nothing is visible it drops to 15 to 19 MB. That is
  about half of every visible footprint, and no phase of this RFC addresses it. It is
  the next thing to investigate: compositing layers, the vibrancy material, and IOSurface
  backing stores at the display's scale.
- Range switching is the largest WebContent driver: on the CPU route, 1 h and 24 h
  alternated three times took WebContent from a 298 MB median to 403 MB, with a 625 MB
  peak. Phase 2 starts there, since the zoom preview measured smaller (503 MB peak over
  five zooms).
- A hidden dashboard keeps 175 MB of WebContent until the 5-minute destroy, while the GPU
  process lets go (300 to 19 MB). Phase 3's condition holds, so it goes ahead.
- The Timeline's 30-day reads take `statglass` from about 50 MB to 75 MB median and
  185 MB peak. Phase 5 starts with the 30-day lane and heatmap queries.
- Tray-only footprint is 75 MB, under the 150 MB target. The tray peak of 339 MB is the
  GPU process at launch (270 MB), which is released within the warm-up.

The baselines in `perf-budget.json` stay empty until five clean runs exist.

## GPU process and lifecycle (2026-11-04, D-127)

Bench-only tools added for this: `STATGLASS_BENCH_EVAL` (a script run in every webview),
`STATGLASS_BENCH_WK_FEATURES` and `STATGLASS_BENCH_WK_DEFAULTS=0` (WebKit feature A/B), and
the `open=<route>`, `popover` and `popover-hide` action steps, which let one launch go from
background through each window and back.

The GPU process's 250 MB is WebKit's accelerated drawing: `footprint` shows 28 allocations
of 8 MB, and a page holding one ticking number reproduces them in either window, at any size.
Hiding content, vibrancy, blur, the chart scroll layers and Performance mode changed nothing;
a static page let them go within seconds. With `AcceleratedDrawingEnabled` off:

| Window | Coalition | GPU process | CPU |
|---|---:|---:|---:|
| Popover, WebKit defaults | 407 to 426 MB | 265 MB | 4.5 to 4.8% |
| Popover, accelerated drawing off | 163 to 180 MB | 8 to 16 MB | 5.0 to 5.3% |
| Overview, WebKit defaults | 555 MB | 281 MB | 9.8% |
| Overview, accelerated drawing off | 324 MB | 19 MB | 15.7% |

So the popover is built with it off and the dashboard keeps it. Setting it on the live webview
as the page loaded raced the first paint (the pool returned every 10 s on some launches);
building the webview with a configuration that carries it held 8 MB on five of five launches.

Lifecycle, one launch each, medians (fix build before the teardown change, then after D-127):

| Phase | Before D-127 | After D-127 |
|---|---:|---:|
| Background, fresh launch | 111 MB | 109 MB |
| Popover closed again | 152 MB | 133 MB |
| Dashboard closed, first minutes | 452 MB for 5 min | 170 MB from 10 s after close |
| Everything closed, settled | 199 MB | 169 MB |

The after-close states settle at 0.3 to 0.5% CPU in both. The "after" run's open-window
phases are not comparable: the machine was in use, and the windows were covered for part of
the run. What still stays resident after everything closes, against a fresh launch: the warm
popover's WebContent (46 to 68 MB once it has been opened), the `statglass` process (40 to
64 MB after the Timeline's 30-day reads, down from 84 with the read pool capped at two idle
connections), and the GPU process (8 to 23 MB).

Open: the history payload is JSON objects per point; a columnar binary response (Tauri's raw
`Response`) would cut parse time and heap, and changes the webview's data types, so it needs
its own decision. WebContent growth on range switching (phase 2) is now bounded by a 30 s `gcTime`
but was not re-measured. The popover with accelerated drawing off has not been compared by
eye against the old rendering.

## Out of scope

The following are out of scope: replacing WKWebView, rewriting SVG charts as canvas, dropping the warm
popover (D-024, and its 86 MB is inside the target), lazy-mounting below-the-fold charts, and switching
to mimalloc. Each is a large change for a gain nobody has measured. If one becomes justified, phase 1's
per-route numbers will show it.

## Alternatives considered

- Shorten `DESTROY_AFTER` to 30 s instead of phase 3. It's simpler, but a reopen inside a minute then
  pays for process startup, route load and a full ring backfill (D-066), which the 5-minute timer
  exists to avoid.
- Destroy the popover webview when idle. That saves about 86 MB tray-only at the cost of a cold popover
  open. It reverses D-024 for memory that is already inside the target.

## Verification

- Phase 1: `make bench` prints each scenario's steady (median), end and peak footprint per process.
  The first full run is recorded in this RFC, and the `footprint` section is populated from it.
- Phase 2: the scenario that produced the spike shows a peak inside its ceiling in five runs.
- Phase 3, if built: the 60 s closed scenario is near idle for the dashboard's processes, the remount
  perf case passes, and reopen latency is reported.
- Phase 4: a render test checks that each call site zeroes its canvas on unmount, and phase 2's
  scenarios are compared before and after.
- Phase 5: `statglass`'s footprint after a dashboard session and close, before and after the fix,
  plus a `memory_gates.rs` case if a bound is added.
- Phase 6: a unit test on the shell's handler, and a manual check that `memory_pressure -S -l warn`
  fires the source and destroys a hidden dashboard.

## Review outcome

A devil's-advocate review against the code, on 2026-11-03, changed the draft as follows:

- The draft said the by-app Memory figure was a peak that read as current usage, and proposed a
  "Peak" label (old phase 7). The screenshot came from the Processes view, which shows current
  `mem_bytes`. The Memory route's by-app table and the CLI already say "Peak". The phase was dropped.
- Phase 1 relied on resetting `ri_interval_max_phys_footprint`, which needs a private call. It now
  reads the lifetime max after the per-scenario relaunch the bench already does. It also gained an
  interaction driver and a seeded bench history, because the bench could only select a route.
- Phase 2's zoom-preview diagnosis was undersized: one subtree is copied, not every lane. The scale
  factor and the uPlot rebuild on `lo`/`hi` are the stronger candidates.
- Phase 3 had been tied to `set_visible`, which also fires on occlusion, and to a JS timer that a
  hidden webview throttles. It now uses a Rust timer from `hide_dashboard`, keeps the default query
  `gcTime` instead of dropping queries, adds a remount perf case, and only goes ahead if phase 1
  shows it's needed.
- Phase 4 now zeroes the canvas through `plot.ctx.canvas`, because uPlot's `destroy()` doesn't.
- Phase 5 (`malloc_zone_pressure_relief`) was dropped. The `heap` evidence shows the growth is live
  allocations, so the phase now finds them.
- Phase 6 named the wrong sysctl (`kern.memorystatus_level` is a free-memory percentage) and would
  have polled a collector series. It now uses the pushed dispatch memory-pressure source.
