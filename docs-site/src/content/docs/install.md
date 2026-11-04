---
title: Install StatGlass
description: Download StatGlass for Apple Silicon Macs running macOS 26 or later. Follow the steps to install the DMG, approve the first launch, or build from source.
group: Getting started
order: 1
featured: true
---

StatGlass runs on Apple Silicon Macs (M1 or newer) with macOS 26 or newer.

## Install the app

1. Download StatGlass_aarch64.dmg from the [latest release](https://github.com/steamfield/statglass/releases/latest).
2. Open the DMG and drag StatGlass into Applications.
3. Eject the DMG, then open StatGlass from Applications.
4. If macOS blocks the first launch, go to System Settings → Privacy & Security → Open Anyway, then confirm Open.

> [!NOTE]
> StatGlass is ad-hoc signed, without Apple notarization, so the first-launch approval is expected. You do not need an Apple Developer account. Managed Macs may restrict this approval; see [Apple's instructions](https://support.apple.com/en-us/102445).

StatGlass appears in the menu bar and asks you to choose a menu bar style. [The menu bar and windows](/docs/menu-bar-and-windows) explains the choices.

> [!NOTE]
> StatGlass was called Kelvo before 0.4.0 and installs as a separate app. Kelvo does not update to it, and its settings and history are not imported. Quit Kelvo and stop its remote agents before switching, then install StatGlass on each host and add the hosts again.

## Build from source

Install the Xcode Command Line Tools (`xcode-select --install`), [Rust](https://rustup.rs) and [Bun](https://bun.sh), then:

```sh title="Terminal"
git clone https://github.com/steamfield/statglass.git
cd statglass
scripts/install.sh
```

Run from a checkout, the script builds that checkout as it is and doesn't pull.

## Uninstall

Quit StatGlass from its menu bar item, then delete the app, its data and the source. The DMG installs into `/Applications` and the source installer into `~/Applications`; the first line removes whichever you have:

```sh title="Terminal"
rm -rf /Applications/StatGlass.app ~/Applications/StatGlass.app
rm -rf ~/Library/Application\ Support/com.tryopendata.statglass ~/Library/Logs/com.tryopendata.statglass
rm -rf ~/.statglass
```

[Updates](/docs/updates) covers keeping an installed copy current.
