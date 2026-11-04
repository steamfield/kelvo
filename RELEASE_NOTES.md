# StatGlass 0.5.0

StatGlass is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

StatGlass now includes a command-line tool, `statglass`, so coding agents and scripts can
ask what happened on this Mac and on your remote hosts. It reads what StatGlass already
collects: current readings, history with its gaps, detector events and the top apps
for a time window. It only reads, and only programs running under your account can
use it.

- Turn it on or off in **Settings → Command line**. Access is on by default.
- **Settings → Command line** also adds the `statglass` command to `~/.local/bin`.
- Run `statglass` with no arguments for a short guide. Run `statglass skill install`
  to teach Claude Code how to use it.
- StatGlass must be running for the command to answer.

## Moving from Kelvo

StatGlass was called Kelvo before 0.4.0. Kelvo does not update to StatGlass
automatically: install StatGlass from the DMG below, then add your remote hosts again
from **Add host**. Kelvo's settings and history are not imported.

## Install

Download **StatGlass_aarch64.dmg**, open it, and drag **StatGlass** into **Applications**.
Eject the disk image, then open StatGlass from Applications.

This release is ad-hoc signed, without Apple notarization. If macOS blocks the first
launch, open **System Settings → Privacy & Security → Open Anyway**, then confirm
**Open**.
