# Diir

A native, cross-platform desktop file explorer built with **Rust + GPUI**.
This repository contains the application foundation and a tested core navigation
contract. The GUI remains an appearance preview; live directory browsing and
visible navigation errors are not wired yet. See [navigation.md](docs/navigation.md).

## Start

Install [Rust through rustup](https://rustup.rs/) and the native prerequisites in
[development.md](docs/development.md). The repository pins its Rust toolchain and GPUI API.

```sh
cargo run -p diir-app
# Use release builds for responsiveness and performance evaluation:
cargo run -p diir-app --release
```

The initialization window contains only the shell, appearance controls, and an appearance preview.
The preview column exposes the tinted native glass while the content panel stays opaque.
Open **Settings → Appearance** to choose **Frosted Glass** (default), **Dracula**,
**Monokai**, or **Windows Explorer**, and to reduce transparency. Windows Explorer
uses a light palette and a different shell: navigation pane, address bar, command
strip, and file column headers. These are appearance previews; no files are listed
and navigation/file commands are not active yet.

Use `Cmd/Ctrl+,` to open Settings, arrows or Tab to navigate, Enter to choose,
Left to return, and Escape to dismiss. Cycle themes with `Cmd+Shift+T` on macOS /
`Ctrl+Shift+T` elsewhere.
Use `Cmd/Ctrl+Shift+G` to toggle reduced transparency. Preferences are session-only for now.
Quit with `Cmd/Ctrl+Q` or close the window.

Frosted Glass requests native vibrancy/acrylic on macOS and Windows. Linux uses
opaque tinted surfaces because compositor blur varies. Disabling transparency
uses opaque surfaces everywhere. Blur availability depends on the OS/compositor;
the theme does not promise an identical effect on every machine. The pinned GPUI
API exposes blur as a mode, without a configurable radius; the stronger frosting
uses a heavier tint, rather than an additional blur filter.

## Layout

| Path | Responsibility |
| --- | --- |
| `crates/diir-core` | UI-independent preferences, palettes, density, contrast tests |
| `crates/diir-app` | GPUI lifecycle, shell composition, actions, platform policy |
| `crates/diir-app/src/components` | Small reusable presentation components |
| `AGENTS.md` | Instructions for contributors and coding agents |
| `docs/architecture.md` | Boundaries and approved decisions |
| `docs/development.md` | Platform prerequisites, checks, manual validation |
| `docs/performance.md` | Performance rules and future measurement protocol |
| `.github/workflows/ci.yml` | Native build/check matrix |

## Checks

```sh
cargo fmt --all --check
cargo test -p diir-core
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

The included `Cargo.lock` records compatibility pins; use `--locked` for reproducible checks.
CI checks all three operating systems. Passing a build does not replace a native
interactive smoke test. Native unsigned packages and separate trusted signing
workflows are described in [packaging.md](docs/packaging.md). Signing requires
certificates and successful verification; configuring a workflow is not a claim
that signed artifacts have been produced. Auto-updates, persistent settings,
and file operations remain outside this checkpoint.


See [validation.md](docs/validation.md) for checks actually completed and remaining platform validation.


Diir is licensed under [Apache-2.0](LICENSE). Packaged dependency notices preserve
third-party license terms.
