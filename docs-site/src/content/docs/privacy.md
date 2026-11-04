---
title: Privacy and data
description: What StatGlass reads, where it stores history and settings, and what leaves your Mac.
group: Privacy
order: 40
---

Local monitoring reads macOS data as your user, without a privileged helper, and keeps everything on your Mac. StatGlass has no backend and sends no metrics, history or machine identifier anywhere.

## Permissions

If you turn on an alert in Settings, macOS asks once whether StatGlass may send notifications. Monitoring another Mac requires Remote Login and SSH key access on that Mac; setup uses a bundled SSH helper and may require native macOS approval there. See [Monitor another Mac](/docs/remote-macs).

## Where data lives

History and settings are in `~/Library/Application Support/com.tryopendata.statglass/`, logs in `~/Library/Logs/com.tryopendata.statglass/`. [Uninstall](/docs/install#uninstall) removes both.

## What leaves your Mac

Update checks contact GitHub (see [Updates](/docs/updates)). If you enable remote monitoring, authorized SSH clients can read that host's metrics and history; a controller keeps a separate local history mirror for each remote host. This sharing uses SSH directly.

## This website

This documentation site is separate from the app, and nothing the app collects is involved. The site sends page views and three named actions to PostHog: `download_clicked`, `demo_started` and `demo_dashboard_opened`. There is no automatic click capture and no session recording. Until you accept the consent banner, and after you decline, PostHog counts without cookies and stores nothing on your device; declining stores only the `sf-consent` cookie set to `denied`, so the banner doesn't ask again. If you accept, PostHog stores an identifier in a cookie on .steamfield.io, shared with the other Steamfield sites, plus a copy in this site's localStorage and sessionStorage. The [Steamfield privacy page](https://steamfield.io/privacy) has the details and lets you change your choice.
