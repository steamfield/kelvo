# StatGlass 0.6.2

StatGlass is a free, open-source system monitor for Apple Silicon Macs running macOS
26 or later.

This release makes the dashboard's tables sortable by every value column.

- Click a column header to sort by it. The first click puts the largest values first,
  or sorts text from A to Z, and a second click flips the order.
- The interface, volume, disk space, largest files and disk process tables can now be
  sorted. On the Disk page, sorting processes by Read or Written now ranks every
  process, not just the top 10 by total.
- The containers and storage tables sort both ways, and the containers page remembers
  the direction. Image, ports, state and restarts can be sorted too.
- Missing readings, such as a stopped container's CPU, always sort to the bottom
  instead of being treated as zero.
- Rows that summarize the rest, such as **Other apps** or **N smaller items**, stay at the
  bottom whatever the sort. On the Disk page, other volumes in the same container (System,
  VM and so on) now sort among your folders by size.
- Live tables keep their order while the pointer is over them, so a row doesn't move out
  from under the cursor.

Tables ranked by StatGlass itself, such as the top apps over a time range, keep their
ranking column.

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
