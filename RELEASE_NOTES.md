# StatGlass 0.4.0

StatGlass is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

Kelvo is now StatGlass. This is the first release under the new name, and it installs as
a separate app with its own settings, history and update channel.

- When a remote host can't connect, the dashboard shows a notice with **Retry
  connection**, **Open Hosts** and expandable diagnostics, instead of an empty page.
- A host that has never connected shows **Connecting** rather than Live or
  "Sampling every 0s", and its charts stay empty until real samples arrive.
- A remote Mac whose screen is locked keeps streaming live readings. Previously the
  controlling Mac stopped showing its incoming data while the display was idle.
- An unexpected error on a dashboard page keeps the sidebar and offers to retry or go
  to another host.

## Moving from Kelvo

Kelvo does not update to StatGlass automatically. Install StatGlass from the DMG below.
Kelvo's settings and history stay where they are and are not imported.

Quit Kelvo and stop its remote agents before switching. Then install StatGlass on each
remote host from **Add host** and add the hosts again. Once you no longer need Kelvo,
delete `/Applications/Kelvo.app`.

## Install

Download **StatGlass_aarch64.dmg**, open it, and drag **StatGlass** into **Applications**.
Eject the disk image, then open StatGlass from Applications.

This release is ad-hoc signed, without Apple notarization. If macOS blocks the first
launch, open **System Settings → Privacy & Security → Open Anyway**, then confirm
**Open**.
