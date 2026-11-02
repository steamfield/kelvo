---
title: The menu bar and windows
description: The menu bar item, the popover, the dashboard and how often each samples.
group: Using StatGlass
order: 10
featured: true
---

StatGlass lives in the menu bar. Click its item to open the popover, a compact column of module cards; open the dashboard window for full charts, history and processes.

## Menu bar styles

The menu bar can show one combined item or one item per module, each as a number, a graph or a per-core strip. StatGlass asks you to pick a style on first launch, and you can change it in Settings.

## The popover

Popover cards collapse individually or together, keep their saved presentation, and resize the popover to fit. Clicking a card opens that module in the dashboard.

## The dashboard

Each module has a page with live charts and its top processes. CPU, GPU, Memory, Power, Network and Disk also support rolling durations and fixed date ranges. Return to Live to resume a moving window, and brush Network or Disk charts to inspect activity within a selection.

The Timeline puts every module on one chart and marks events on it. [History and storage](/docs/history#the-timeline) covers it.

## Sampling and resource use

With only the menu bar showing, StatGlass samples every 2 seconds and uses under 1% of one core. Open a window and it samples every second (adjustable from 0.5 to 60 seconds) and uses 6 to 10%, depending on the page.
