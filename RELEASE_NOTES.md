# StatGlass 0.4.1

StatGlass is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

- The Linux agent no longer runs out of memory on busy hosts. With a dashboard open on
  a host that starts many short-lived processes, the agent could grow past its service
  memory limit and be stopped by the system. It now keeps less recent history in
  memory once that history is saved to disk.

Linux hosts with automatic updates enabled pick this up from their daily update
check. Others can update from **Hosts → Update agent**.

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
