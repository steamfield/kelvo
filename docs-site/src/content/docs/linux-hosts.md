---
title: Monitor a Linux host
description: Install the bundled StatGlass agent on a Linux server and monitor it from your Mac.
group: Remote hosts
order: 31
---

In Hosts, add the Linux server using its SSH alias or `user@address`, then choose Check connection. The same SSH key and host verification requirements apply as for [another Mac](/docs/remote-macs#set-up-access). StatGlass bundles static agents for x86_64 and aarch64 Linux; the Mac app remains the controller. Linux setup requires systemd.

## Installing the agent

Review the detected system, installation path, service account and history budget before installing. A normal account uses a systemd user service; a root SSH account uses a system service. If lingering is off, setup shows the command needed to keep a user service running after logout.

## What you see

Select the host to view live metrics, stored history, per-app usage and disk space reports. Linux memory charts show used and cached memory. Containers show cgroup services and containers where available. Sensor, power and container details depend on the host's hardware and read permissions.

## Agent updates

Older agents need one Update agent action to gain automatic updates. After that, each Linux host checks and installs signed compatible releases independently through systemd. Host settings show its update preferences and results.

If a server is reinstalled or replaced, Re-pair adds its new identity and keeps the old mirrored history as an offline host. Host settings also provide agent removal.
