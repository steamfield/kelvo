# Kelvo 0.3.2

Kelvo is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

This patch makes it clear when connected hosts are running an older Kelvo than this
Mac, and lets you update them from one place.

- The Hosts page shows each host's Kelvo version and, when it is behind, the version
  this Mac can install, with a link to what's new. Offline hosts show the version
  they last reported.
- When hosts fall behind, the Hosts sidebar entry shows a count and dashboard pages
  show a notice linking to Hosts. Dismiss it and it stays hidden until the next
  version. If this Mac has an update pending, the notice asks you to update this Mac
  first.
- **Update all** on the Hosts page updates every reachable Linux host in turn.
- A host too old to connect now shows its version and can be updated from its row.

Mac hosts are updated by installing Kelvo on them again from **Add host**.

## Updates

Kelvo 0.3.0 and later install this update automatically when automatic installation
is on, or through **Settings → General → Check now**. History and settings are
preserved.

## Install

Download **Kelvo_aarch64.dmg**, open it, and drag **Kelvo** into **Applications**.
Eject the disk image, then open Kelvo from Applications.

This release is ad-hoc signed, without Apple notarization. If macOS blocks the first
launch, open **System Settings → Privacy & Security → Open Anyway**, then confirm
**Open**.
