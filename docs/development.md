# Development

## Toolchain

Install rustup from https://rustup.rs/. `rust-toolchain.toml` pins Rust 1.100.0,
rustfmt, and Clippy. GPUI 0.2.2 and the resolved dependency graph are pinned in `Cargo.lock`.
Do not copy current upstream GPUI examples blindly: its main branch may have
different platform-crate APIs from the published version used here.

## Native prerequisites

### macOS

- Install the full Xcode application, open it, and install its macOS components.
- Install command-line tools with `xcode-select --install` if needed.
- Select Xcode: `sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer`.
- Install the Metal toolchain if Xcode reports it missing: `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -downloadComponent MetalToolchain`.
- If Command Line Tools remain selected, prefix Cargo commands with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` to use full Xcode for that command.
- A Metal-capable desktop is required for an interactive run.

### Linux (Ubuntu/Debian)

```sh
sudo apt-get update
sudo apt-get install -y build-essential clang cmake pkg-config \
  libasound2-dev libfontconfig1-dev libfreetype6-dev libwayland-dev \
  libx11-dev libx11-xcb-dev libxcb1-dev libxkbcommon-dev \
  libxkbcommon-x11-dev libssl-dev libzstd-dev libvulkan-dev
```

Use a desktop session with a working Vulkan driver, installed fonts, and X11 or
Wayland. Package names vary by distribution. Native blur is disabled in Diir's
Linux policy; the default Frosted Glass palette remains available as opaque tint.

### Windows

- Install Rust with the `stable-msvc` host toolchain.
- Install Visual Studio 2022 Build Tools: Desktop development with C++, MSVC, and a Windows SDK.
- Install the LLVM tools if native dependency build scripts request them; ensure `clang` is on PATH.
- Use a desktop with graphics drivers supported by GPUI. Acrylic depends on Windows composition/settings.

## Commands

```sh
cargo run -p diir-app
cargo fmt --all --check
cargo test -p diir-core
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p diir-app --release
```

Use `--locked` for test/check/clippy/build commands to keep the verified dependency graph.
No credentials, GitHub connection, or external application service is required to run the shell.

## Native smoke test checklist

- Launch and close the Diir window on each target OS; confirm the process exits.
- Switch all three themes using mouse controls and the keyboard shortcut.
- Toggle reduced transparency and return to the default theme.
- Check readable foregrounds and visible boundaries over both white and black desktop backgrounds.
- Resize down to the minimum size and check controls at 100%, 150%, and 200% scaling.
- Confirm no continuous redraw/CPU activity while idle.
- Verify Linux X11 and Wayland separately, plus macOS and Windows blur/fallback behavior.

CI compiles and lints on native runners. It does not verify GPU/compositor output,
package installers, signing, or interactive accessibility.


## Dependency compatibility

The lockfile retains `libc 0.2.186` and `mio 1.2.0`. GPUI 0.2.2's
`gpui_http_client -> zed-async-tar -> xattr 0.2.3` dependency uses Linux's legacy
`libc::ENOATTR` alias, which is absent in the newer libc resolution tested during
initialization. Newer mio requires a newer libc, so both versions must move together.
Do not run an unrestricted `cargo update` without repeating the native matrix.
Prefer an upstream dependency fix when upgrading rather than modifying Cargo's cache.
