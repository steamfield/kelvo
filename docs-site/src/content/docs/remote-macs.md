---
title: Monitor another Mac
description: Add another Apple Silicon Mac over SSH and see its live metrics and mirrored history.
group: Remote hosts
order: 30
featured: true
---

Open Hosts → Add Mac in a packaged StatGlass build. Both Macs need Apple Silicon and macOS 26 or newer. StatGlass connects over SSH using the account signed in to the other Mac's desktop.

## Set up access

1. On the other Mac, enable System Settings → General → Sharing → Remote Login and allow that account.
2. On this Mac, set up SSH key access. The in-app SSH setup guide explains how to create a key, authorize its public key on the other Mac, and unlock it in your SSH agent. StatGlass's connections cannot prompt for passwords or key passphrases.
3. Enter an SSH alias from `~/.ssh/config` or `user@address`. A config file is optional; StatGlass detects when it is missing and opens the guide.
4. Choose Check connection. If the host is unknown, choose Verify host and compare its fingerprint with the other Mac before trusting it. Then connect.

Setup reuses or copies the packaged StatGlass app and enables remote monitoring and launch at login on the other Mac. macOS approvals must be completed there.

## What you see

Select the Mac in Hosts to see its live metrics and mirrored history. Settings, alerts, process control and menu-bar metrics remain local. Each remote host gets its own history mirror on this Mac, separate from local [history](/docs/history).

## When a connection fails

Follow the recovery message and expand its details when needed. For a saved Mac, open its Settings → Show SSH setup guide for verification and access-test commands, save any address changes, then use Retry in Hosts. Recheck config only checks for the local file; it does not test SSH access.

Linux servers are covered in [Monitor a Linux host](/docs/linux-hosts).
