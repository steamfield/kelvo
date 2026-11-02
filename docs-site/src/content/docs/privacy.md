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

This documentation site is separate from the app. It can count page views and download clicks with PostHog, configured without cookies or other stored identifiers, without automatic click capture and without session recordings. Nothing the app collects is involved.
