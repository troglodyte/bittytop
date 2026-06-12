# bittytop Documentation

This directory contains technical documentation about the internal workings of `bittytop`.

## Formatting and Metrics
The following documents explain where to find the formatting logic and how data is presented for each metric:

- [Network Monitoring](network.md) - Details on `--wtn` and global network metrics.
- [CPU Monitoring](cpu.md) - Details on global and per-process CPU usage.
- [Memory Monitoring](memory.md) - Details on global and per-process memory usage.
- [GPU Monitoring](gpu.md) - Details on GPU load and system-wide GPU tags.

## Core Logic
Visual formatting is centrally managed in `src/view.rs`, while data collection is handled by the `MonitorService` in `src/service.rs`.
