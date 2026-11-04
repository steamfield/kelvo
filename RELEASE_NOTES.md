# StatGlass 0.6.1

StatGlass is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

This release cuts StatGlass's memory use, most of all while the popover is open and
after you close the dashboard.

- With the popover open, StatGlass measured about 170 MB in all, down from about
  420 MB. Most of the difference was graphics memory WebKit kept for the popover's
  once-a-second redraw.
- Closing the dashboard now releases it after 10 seconds instead of 5 minutes, so the
  app is back to about 170 MB within seconds. Opening it again takes a fraction of a
  second.
- History a chart no longer shows is dropped after 30 seconds instead of 5 minutes,
  and history reads keep fewer database connections open.

The `statglass` command-line tool also gets a few additions:

- `--from` and `--to` accept a local date and time, such as `2026-11-03 15:00`, or a
  date alone for midnight.
- `statglass events --summary` counts a range's events per kind.
- A remote host's last minutes that have not synced yet are labelled as such, and a
  mistyped series label suggests the closest stored series.

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
