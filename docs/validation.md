# Initialization validation — 2026-11-25

Environment: Debian 13, x86_64 Linux, Rust 1.100.0, GPUI 0.2.2.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `cargo test -p diir-core --locked` | Passed: 3 behavior/contrast tests |
| `cargo check --workspace --all-targets --locked` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo build -p diir-app --locked` | Passed with local linker search path described below |
| Interactive GPU/window smoke test | Not run: no desktop display session in the build environment |
| macOS and Windows native checks | Not run locally; CI matrix configured, not executed |
| Heavy-load benchmarks | Not run; no workload features are included in initialization |

The environment has the XCB/xkbcommon runtime libraries but lacks their unversioned
development linker names, and system package installation was unavailable. The
link check used temporary `.so` links to the installed system libraries via
`LIBRARY_PATH=/tmp/diir-native-link`. No repository build flags or dependencies
were changed for this environment. A normal desktop setup should install the
development packages listed in `development.md` instead.

The initial unrestricted dependency resolution failed in upstream `xattr 0.2.3`
against newer libc. The included lockfile resolves this with compatible libc/mio
versions; see `development.md`. Cargo also reports a future-compatibility notice
for upstream `proc-macro-error2 2.0.1`; the current build and Clippy checks pass.

Theme settings currently last for one process session. Native compositor effects,
system reduced-transparency preferences, packaging, signing, and update delivery
are not validated by this checkpoint.


## Frosted Glass preview revision — macOS, 2026-11-25

Environment: Apple Silicon macOS, Rust 1.100.0, GPUI 0.2.2; full Xcode selected
per command with `DEVELOPER_DIR`. Installed Xcode's missing Metal toolchain.

- Passed: formatting check, all 3 core behavior/contrast tests, workspace check
  with all targets, Clippy with all targets and warnings denied, and debug build.
- Glass compositing tests now also check accent text and border contrast against
  black and white backdrops. Brighter Frosted Glass foreground tokens support
  the reduced 190/255 outer tint.
- Launched the revised build using a temporary local app wrapper; the native app
  inventory reports it running. UI inspection timed out, so visual translucency,
  theme switching, shortcuts, scaling, and backdrop checks remain unverified.
- Linux/Windows builds and the native CI matrix were not run for this revision.
- Cargo retains upstream future-compatibility notices for `block 0.1.6` and
  `proc-macro-error2 2.0.1`; no dependency versions were changed.


## Appearance menu and Windows Explorer theme — macOS, 2026-11-25

- Passed: formatting, 4 core tests (including menu navigation and all-theme
  contrast), all-target workspace check, and Clippy with warnings denied.
- Added Settings → Appearance, keyboard menu navigation, active-theme checkmarks,
  reduced-transparency selection, and a Windows Explorer appearance layout.
- Frosted tint is now 208/255. Native blur radius is unchanged: GPUI 0.2.2 exposes
  only the blur mode, without a radius/material setting. No additional native
  integration dependency, unsafe code, or repeated blur filters were introduced.
- Linux/Windows builds and the native CI matrix have not run for this revision.

- Debug rebuild passed and the revised executable was launched on macOS. The
  native UI automation pipe failed to start, so menu clicking, visual theme
  switching, blur appearance, scaling, and backdrop checks remain unverified.


## Native packaging infrastructure — 2026-11-25

Baseline GitHub run [45652798188](https://github.com/Mfuon2/diir/actions/runs/45652798188)
passed all format/core-test/check/Clippy/debug-link steps on Ubuntu 24.04 and
Windows 2022. The macOS 14 job was cancelled because that runner is retired; this
revision replaces it with supported macOS 15 and adds native package builds. That
baseline is not a passing complete matrix.

Local infrastructure checks: Python packaging unit tests, Python syntax checks,
`actionlint`, and `shellcheck` passed. Actual native package/installer/signing
results must be recorded after their jobs run. Real code-signing certificates and
notarization credentials are not configured, so signing is **not validated**.
No Linux graphical session or Windows/macOS cross-platform UI smoke test is
claimed by package metadata or installation checks.

An actual locally optimized Apple Silicon executable was staged into `Diir.app`,
archived, and checked for bundle identifier, executable/license/notices/manifest
payloads, SHA-256 integrity, and unsigned status. Its `otool -L` output contained
only system framework/library paths. This is packaging validation, not a GUI or
signing/notarization test.
