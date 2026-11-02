---
title: Updates
description: How StatGlass checks for, verifies and installs new releases.
group: Getting started
order: 2
---

Open Settings → General → Check now. When a new release is available, choose Install and restart. StatGlass verifies the download's updater signature before installing it, restarts, and keeps your history and settings.

## Automatic updates

With Check for updates automatically enabled, StatGlass checks GitHub shortly after launch and roughly daily while running, with up to 30 minutes of jitter. Install updates automatically is on by default: StatGlass verifies signed downloads, waits until interactive windows are closed and maintenance has finished, then installs and restarts.

Automatic installation never requests administrator privileges; Settings offers a manual installation when needed. Turn off automatic installation to review each release yourself. Turning off automatic checks also stops automatic installation, and Check now still works.

Checks contact GitHub. StatGlass sends no metrics, history or machine identifier; see [Privacy and data](/docs/privacy).

## When an update fails

Use Check now to retry, or download the latest DMG and replace the app after quitting StatGlass. Run StatGlass from Applications, not from the mounted DMG. Older source builds without updater support need this first DMG installation: quit the old copy and replace it in its existing Applications folder (the source installer used `~/Applications`) so you do not keep two copies.
