---
name: statglass
description: Answers questions about CPU, memory, disk, temperature, power, network, processes or detector events on this Mac or the user's other machines (remote Macs, Linux hosts), now or in the past, through the read-only statglass CLI. Use when the user asks what was slow, hot, busy or full, what happened at a time, which host is least busy, or whether a disk is filling up, instead of running top, df or ssh on the current moment.
---

# statglass

StatGlass records 1 s samples, weeks of rolled-up history, detector events and per-app
usage for this Mac and every host it watches, including hosts that are offline now. The
`statglass` command asks the running app; it never changes anything.

Start with `statglass` (no arguments) for the guide, or follow this loop:

1. `statglass hosts` for host names and state.
2. `statglass events --since 3h` and `statglass top --since 1h` to find what happened.
3. `statglass history '<series>' --since 3h` for the trend, with gaps.
4. `statglass now` (or `--all`) for the current moment's headline series; `--detail` for
   every series.

Add `--host <name>` for another machine and `--json` for one JSON object.

Before concluding that nothing happened, read the freshness line and every gap line (and the
`more gaps` summary line): a gap is unknown data, never zero. Single-quote series with braces. Exit 6 means ask the user
(StatGlass not running or not installed, command-line access off, StatGlass could not open its
socket, StatGlass older than this tool, or it does not support the request); exit 5 means the
data is not there.

Read `reference/statglass.md` for series syntax and quoting, time flags, freshness, gaps
and every exit code.
