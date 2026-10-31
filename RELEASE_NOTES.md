# Kelvo 0.3.1

Kelvo is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

This patch fixes history being cut short after 0.3.0. Recorded app usage stored a
detail row for every process every 10 seconds, including the hundreds that were
doing nothing, and grew history by roughly 80 MB a day. With a small size limit,
Kelvo trimmed older history to make room, and resource charts could fall back to
the last day.

- Per-process usage detail is now kept only for processes that used at least 0.1%
  of a CPU core, any GPU, or 100 MB of RAM. App totals and rankings
  still include every process.
- When history is over the size limit, Kelvo now removes the oldest process
  details and recorded app usage first. Resource charts are trimmed only if that is not enough.
- Live charts do less animation work, and historical usage reads repeat less
  database work for apps with many helper processes.

History already trimmed by the size limit cannot be recovered. If you lowered the
limit below the 750 MB default, consider raising it in **Settings**.

## Updates

Kelvo 0.3.0 installs this update automatically when automatic installation is on,
or through **Settings → General → Check now**. History and settings are preserved.

## Install

Download **Kelvo_aarch64.dmg**, open it, and drag **Kelvo** into **Applications**.
Eject the disk image, then open Kelvo from Applications.

This release is ad-hoc signed, without Apple notarization. If macOS blocks the first
launch, open **System Settings → Privacy & Security → Open Anyway**, then confirm
**Open**.
