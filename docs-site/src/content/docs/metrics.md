---
title: What StatGlass measures
description: Every module StatGlass shows, from CPU and GPU to battery, processes and containers.
group: Metrics
order: 20
featured: true
---

StatGlass groups its readings into modules. Each one has a popover card and a dashboard page.

| Module              | What you get                                                                                    |
| ------------------- | ----------------------------------------------------------------------------------------------- |
| CPU                 | Total and per-core load, efficiency and performance clusters, top processes                     |
| GPU                 | Utilization, frequency, GPU time per process                                                    |
| Memory              | App, wired, compressed and cached memory, pressure, swap                                        |
| Power & Sensors     | Power draw with per-app lines, thermal zone history, temperatures, fan speeds and thermal state |
| Network             | Rates per interface, download and upload per process                                            |
| Disk                | Read and write rates, free space, top processes and an on-demand space report with a treemap    |
| Battery             | Charge, health, cycle count, power in or out, time remaining                                    |
| Processes           | Every process, with CPU, memory, disk, network and GPU columns                                  |
| Containers (opt-in) | Local Docker CPU, memory and I/O, history, logs, storage inventory and reviewed cleanup         |

## Network per process

macOS only shows an app the network traffic of processes owned by the same user. System services run as other users, so their traffic counts toward the interface totals but isn't listed per process.

## Containers

Enable monitoring from Containers and choose one local Docker engine. StatGlass reads the Unix socket directly and needs no Docker CLI. Up to 128 containers are monitored at once; larger inventories show explicit partial coverage. Container memory and the macOS runtime footprint are separate measurements. Logs stay in memory while viewed.

Storage & cleanup scans images, writable layers, volumes and daemon build cache on demand, with a five-minute cache. Review stopped containers and unused resources by size and project, then confirm selected actions. Volumes require a separate data-loss acknowledgement, and bind-mounted host files are never removed.

Past readings for every module are kept as [history](/docs/history).
