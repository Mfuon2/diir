# Diir — project instructions

## Product and scope
- The application is **Diir**, a native desktop file explorer for macOS, Linux, and Windows.
- The approved stack is **Rust + GPUI**. Do not introduce a webview or another UI framework.
- This checkpoint is initialization only. Do not fetch, import, prioritize, or implement backlog tasks unless the user requests that work.
- Do not create GitHub issues, change project boards, push, publish, or deploy as a side effect of local initialization.

## Architecture and readability
- Keep platform-independent state and behavior in `crates/diir-core`; it must not import GPUI.
- Keep GPUI entities, event handling, and composition in `crates/diir-app`.
- Extract reusable UI into small, named components under `components/`. Keep root rendering readable.
- Keep operating-system conditionals and native integration at the platform boundary.
- Prefer concrete, small types and explicit ownership. Introduce traits only when they serve an actual boundary.
- Use `Path` / `PathBuf` and native OS strings for future file paths. Never assume UTF-8 paths or Unix separators.
- Handle recoverable errors with results and clear user-visible states; avoid production `unwrap` / `expect` and unsafe code.
- Pin GPUI deliberately and commit `Cargo.lock`. Do not mix APIs from moving upstream main with the pinned release.

## Design system
- **Frosted Glass is the default**; **Dracula** and **Monokai** are switchable alternatives.
- Use semantic palette and density tokens; do not scatter raw colors, spacing, or row heights across features.
- Favor dense, calm layouts: 28 px rows/controls, 13 px body text, restrained spacing and corner radii.
- Glass must not compromise legibility. Keep content surfaces opaque, tint the outer glass heavily, and draw clear control boundaries.
- Use the OS-native blur path where supported. Keep an opaque fallback and a reduced-transparency option.
- Do not simulate expensive blur using repeated filters, per-row translucency, animated noise, or full-window redraw loops.
- Preserve keyboard access, visible selection/focus, and readable text. Essential state must not rely on color alone.

## Speed and heavy workloads
- No filesystem scans, networking, hashing, parsing, blocking waits, or persistence writes in render or UI event handlers.
- Move expensive work to background tasks. Use bounded queues, cancellation, batching, and backpressure as features arrive.
- Virtualize large lists and trees; never construct every row for large directories. Maintain stable identities.
- Avoid cloning entire collections, repeated sorting, and decoding thumbnails during rendering.
- Bound caches and concurrency. Release unused resources; do not hide unbounded growth behind caching.
- Keep idle CPU use low and only invalidate affected UI. Prefer event-driven updates over polling.
- Measure release builds using representative large directories; record hardware, data size, p50/p95, memory, and limitations. Never invent benchmark results.

## Validation and handoff
- Run `cargo fmt --all --check`, `cargo test -p diir-core`, and `cargo clippy --workspace --all-targets -- -D warnings` for relevant changes.
- Run `cargo check --workspace --all-targets` and the native CI matrix for platform-dependent work.
- Add meaningful behavior, contrast, cancellation, or boundary tests when needed; avoid tests that just repeat constants or implementation details.
- Review theme switching, keyboard shortcuts, scaling, and readability on light/dark desktop backdrops when a GUI is available.
- macOS, Linux, and Windows support is a requirement, not a claim of testing. State exactly which builds and UI checks actually ran.
- Keep `README.md` and `docs/` aligned with code. Report changes, validation, and concrete blockers concisely.
