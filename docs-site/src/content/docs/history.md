---
title: History and storage
description: How long StatGlass keeps history, how much disk it uses and what goes first at the limit.
group: Using StatGlass
order: 11
featured: true
---

StatGlass records every module's readings locally, with up to 30 days available depending on your size limit and recorded activity.

## The Timeline

The Timeline puts every module on one chart covering the last hour, 24 hours, 7 days or 30 days, and marks events on it: fans ramping, thermal state changes, a process pinning the CPU, power spikes. Select an interval to zoom in, inspect its events and carry the range into module charts; Back restores the previous range.

Recorded app usage shows the top ten app groups, sortable by CPU, GPU, RAM, estimated CPU power or network usage, with expandable process details and comparison with the previous interval. A heatmap shows CPU load or temperature by hour across available history. Any range exports to CSV.

## How much disk it uses

The default history size limit is 750 MB; change it in Settings. StatGlass keeps resource aggregates in 10-second buckets for 24 hours, one-minute buckets through 7 days and 15-minute buckets through 30 days. The amount of activity and your size limit determine how much history fits, so 30 days is not guaranteed.

Per-process detail is kept only for processes that used at least 0.1% of a CPU core, any GPU, or 100 MB of RAM; app totals include every process. When history reaches the limit, the oldest process details and recorded app usage go first, and resource charts are trimmed only after that.

## Gaps

When the Mac sleeps or StatGlass isn't running, nothing is recorded, and charts show that time as a gap rather than drawing a line across it.

[Privacy and data](/docs/privacy) lists where the history files live.
