# Performance Benchmark Protocol

This document defines the repeatable benchmark harness for large-diff performance work.

## Scope

The harness tracks and gates these metrics:

- `TTFD` (time to first diff content) in milliseconds.
- `selected_file_latency_ms` (selected file first-paint proxy) in milliseconds.
- `scroll_fps_avg` (average synthetic scroll frame-rate proxy).
- `scroll_fps_p95` (95th percentile synthetic per-frame FPS).

## Default Fixture

- Changed files: `50`
- Lines per file: `10000`
- Language mode: `ts`

Generate fixture manually:

```bash
./scripts/create_large_diff_repo.sh --lines 10000 --files 50 --lang ts --force
```

## Run Harness

```bash
./scripts/run_perf_harness.sh
```

The harness runs `tests/performance_harness.rs` in release mode and prints:

- `changed_files`
- `total_core_rows`
- `total_code_rows`
- `scroll_sample_rows`
- `ttfd_ms`
- `selected_file_latency_ms`
- `full_stream_ms`
- `scroll_fps_avg`
- `scroll_fps_p95`

## Thresholds (Default)

- `TTFD <= 300 ms`
- `selected_file_latency_ms <= 800 ms`
- `scroll_fps_p95 >= 115`

If a metric crosses threshold, harness exits non-zero.

## Tuning / Overrides

You can override thresholds and workload parameters:

```bash
./scripts/run_perf_harness.sh \
  --max-ttfd-ms 350 \
  --max-selected-ms 900 \
  --min-scroll-fps 105 \
  --scroll-frames 300
```

Use `--no-gate` to collect metrics without failing:

```bash
./scripts/run_perf_harness.sh --no-gate
```

## Notes

- `scroll_fps_avg` and `scroll_fps_p95` are deterministic CPU-side proxies based on diff row segment generation and cache behavior.
- It is intended for regression detection and cross-branch comparison; it is not a replacement for on-screen FPS validation in the GUI.
- Harness scripts currently require a Unix-like shell environment (`bash`).
