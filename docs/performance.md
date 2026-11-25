# Performance rules

Responsiveness under heavy workloads is an architecture constraint from the start.
No performance benchmark or large-directory workload is implemented in this setup.

- Keep render work proportional to visible content, not total directory size.
- Use GPUI virtualized lists for large collections when file views are introduced.
- Perform directory enumeration, metadata, sorting, search, hashing, and thumbnail decoding off the UI thread.
- Bound work queues, worker concurrency, thumbnail sizes, and caches.
- Cancel obsolete work when navigation or queries change; discard stale responses.
- Batch incoming changes and debounce expensive derived views; preserve stable row IDs and selection.
- Prefer demand-driven metadata and incremental results over eager full-tree work.
- Keep glass at the window boundary; never blur individual rows or animate blur while idle.

## Measurement protocol for future features

Evaluate release builds on recorded hardware and OS/graphics drivers. Use local
directories with 1k, 10k, and 100k entries, including long/native filenames, mixed
metadata, inaccessible items, and slow/network storage where relevant. Record cold
and warm runs separately, sample count, p50/p95 time-to-first-results, full-load
time, input latency during work, peak/idle memory, and cancellation latency.

Frame time at 60 Hz (16.7 ms) is a design target, not a measured guarantee.
Set numeric resource budgets from the first real workload baseline. Keep both
successful and adverse cases in performance reports. Do not add dependencies or
complex caches based on intuition without measurements.
