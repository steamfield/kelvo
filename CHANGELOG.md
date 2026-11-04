# Changelog

## [0.6.0](https://github.com/steamfield/statglass/compare/v0.5.0...v0.6.0) - 2026-11-04

### Changed

- StatGlass uses far less memory. The popover no longer makes the WebKit GPU process
  hold about 250 MB while it is open: the whole app measured about 170 MB with the
  popover open, from about 420 MB. A closed dashboard is released after 10 seconds
  instead of 5 minutes, so closing it brings the app back to about 170 MB within
  seconds instead of holding 450 MB for 5 minutes. History the dashboard no longer
  shows is dropped at once, and history reads keep fewer database connections open.

### Added

- `statglass` commands accept a local date and time for `--from` and `--to` (`2026-11-03 15:00`, or `2026-11-03` for midnight) in this Mac's timezone.
- `statglass events --summary` counts a range's events per kind, with the processes
  named most often and a count per day.
- `statglass history` reports the width of each point slot and prints a "no data"
  row for empty slots.

### Fixed

- A remote host's last minutes not yet synced are labelled not synced instead of no
  data.
- A mistyped series label (`disk.read{dev=disk9}`) is an error with the closest
  stored series suggested, instead of an empty answer.
- Text output dates every time when a range reaches past today, and `top` says
  whether it shows average CPU or peak memory.

## [0.5.0](https://github.com/steamfield/statglass/compare/v0.4.1...v0.5.0) - 2026-11-03

### Added

- A bundled, read-only `statglass` command-line tool for agents and scripts. It
  answers `hosts`, `now`, `metrics`, `history`, `events` and `top` for this Mac and
  remote hosts, with JSON, agent and human output, an error envelope with a next step
  and stable exit codes, `schema` for JSON Schemas, `doctor`, and `skill install`.
  It asks the running app over an owner-only socket and never writes.
- Settings → Command line: an "Allow command-line access" toggle (on by default),
  installing or repairing the `~/.local/bin/statglass` link, and the skill install
  command.

## [0.4.1](https://github.com/steamfield/statglass/compare/v0.4.0...v0.4.1) - 2026-11-03

### Fixed

- The Linux agent's memory stays bounded on hosts that start many short-lived
  processes while a dashboard is open. Recent usage buckets leave memory once they are
  saved, and saved container buckets no longer stay for the full hour. Before, the
  agent could exceed its service memory limit and be killed by systemd-oomd.

## [0.4.0](https://github.com/steamfield/statglass/compare/v0.3.2...v0.4.0) - 2026-11-03

### Changed

- Kelvo is renamed to StatGlass. The app, Linux agent, bundle identifier, data and
  log paths, services and release files use the new name. StatGlass installs beside
  Kelvo, does not import its settings or history, and does not replace it through the
  updater.

### Fixed

- A remote host that fails to connect shows a notice with Retry connection, Open
  Hosts and diagnostics, and dashboard render errors keep the sidebar with retry and
  host navigation.
- A never-connected host shows Connecting and keeps its charts empty until real
  samples arrive, instead of reporting a 0 s cadence.
- A remote Mac with a locked or idle display keeps streaming to the controller. The
  controller applied the remote display-idle flag to its own stream delivery.

## [0.3.2](https://github.com/steamfield/kelvo/compare/v0.3.1...v0.3.2) - 2026-11-01

### Added

- Each host on the Hosts page shows the Kelvo version it runs and, when it is
  behind, the version this Mac can install, with a link to the release notes. An
  offline host shows the version it last reported.
- When hosts are behind this Mac, the Hosts sidebar entry shows a count and
  dashboard pages show a dismissible notice linking to Hosts. If this Mac has its
  own update pending, the notice asks to update this Mac first.
- Update all pushes the update to every reachable Linux host in turn, after a
  confirmation that lists them, and reports one result list.

### Fixed

- A host whose Kelvo version is too old to connect now shows its version and can
  be updated from its row.
- Dashboard pages animate in section by section again.

## [0.3.1](https://github.com/tryopendata/kelvo/compare/v0.3.0...v0.3.1) - 2026-10-30

### Fixed

- Recorded app usage no longer stores per-process detail for idle processes, which
  made history grow by roughly 80 MB a day. Under a small size limit that growth
  trimmed all older history, including resource charts, back to the last day.
  Processes are kept when they use at least 0.1% of a CPU core, any GPU, or at
  least 100 MB of RAM. App totals still include every process.
- When history is over the size limit, Kelvo now removes the oldest process
  details and recorded app usage first, and trims resource chart history only if
  that is not enough.

### Changed

- Reduced live-chart animation work by removing forced layout reads and keeping
  chart motion on separate drawing layers. Sampling and history detail are unchanged.
- Reused app identity lookups within each historical usage bucket, reducing
  repeated database work for helper processes.

## [0.3.0](https://github.com/tryopendata/kelvo/compare/v0.2.0...v0.3.0) - 2026-10-26

### Added

- Automatic installation of signed releases on Macs and Linux hosts, with
  per-host controls and update results. Macs wait until interactive windows are
  closed; Linux agents update independently through systemd.
- Timeline range selection, animated zoom and Back navigation, shared with module
  charts, plus focused event annotations.
- Recorded app usage for historical ranges, with top-ten app groups, expandable
  process details and comparison with the previous interval. Groups can be sorted
  by CPU, GPU, RAM, estimated CPU power or network usage.
- Saved compact popover cards, individual collapse controls and Collapse all /
  Expand all. The popover resizes to fit its content.

### Changed

- Stored app attribution and coverage alongside resource history, with older
  intervals kept at progressively coarser resolution. App attribution starts
  recording after the upgrade; earlier unrecorded app usage is unavailable.
- Raised the default history size limit to 750 MB. Existing valid saved limits
  are preserved; retention still depends on recorded activity and the size cap.

### Fixed

- Temperature defaults now follow macOS regional preferences while preserving
  saved choices. Battery history respects the module toggle, and unavailable
  Battery routes return to a supported page. ([#1](https://github.com/tryopendata/kelvo/issues/1))
- Memory pressure, usage and swap sections now use the full content width.
- Short and ongoing pauses no longer cover charts with prominent captions;
  pause details remain available on hover and keyboard focus.
- Historical selections retain their dates as live data arrives, recover after
  failed reads, and survive returning from container details.

## [0.2.0](https://github.com/tryopendata/kelvo/compare/v0.1.0...v0.2.0) - 2026-10-25

### Added

- Linux host monitoring over SSH, with bundled x86_64 and aarch64 agents, systemd
  installation, updates, removal and identity re-pairing.
- Linux CPU, memory, disk, network, hardware sensor, battery and power collectors
  where available, plus cgroup service and container monitoring.
- Remote per-app history and disk space reports.
- Rolling and fixed date ranges across module charts, with selection summaries.
- On-demand disk space reports with a drillable treemap.
- Per-app power draw lines and thermal zone history.

### Fixed

- Updated TLS certificate verification, Tokio, Vite and the test DOM dependency
  to resolve dependency audit findings.

- Remote feed replacement, reconnects after sleep, backoff and agent restarts.
- Loading stored remote history into live charts and preserving disk selections
  through process navigation.
- Linux agent memory accounting, stable sensor naming and counter reset handling.
- Repeated rendering in power, container and app usage views.
- Hidden volume heading layout and accessibility coverage.

## [0.1.0](https://github.com/tryopendata/kelvo/releases/tag/v0.1.0) - 2026-10-16

- First downloadable Apple Silicon release for macOS 26 or later.
- CPU, GPU, memory, power, sensors, network, disk, battery and process monitoring.
- Local history, Timeline, heatmaps, CSV export and alerts.
- Optional local Docker and remote Mac monitoring.
- Ad-hoc-signed DMG and signature-verified in-app updates.
