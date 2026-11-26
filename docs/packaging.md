# Native packages and signing

Diir uses Rust and GPUI throughout. These workflows build application artifacts;
they do not create public GitHub Releases, publish package repositories, or change
which program owns file/directory associations.

## Outputs and supported build baselines

| Output | Architecture | Build and package baseline |
| --- | --- | --- |
| `Diir.app` inside a ZIP | Apple Silicon / arm64 | macOS 15, native GitHub runner |
| `Diir.app` inside a ZIP | Intel / x86-64 | macOS 15 Intel, native GitHub runner |
| `diir.exe` inside a ZIP | x86-64 | Windows Server 2022 / MSVC runner |
| `.deb` | amd64 | Debian 13 container |
| `.rpm` | x86-64 | Fedora 45 container; installed with DNF |

These are initial build targets, not claims of interactive verification on every
system. The Fedora RPM is not advertised as RHEL/Rocky-compatible; those need
separate native baselines. Linux containers cannot prove GPU/compositor behavior.
The Windows ZIP is a portable application, not an MSI installer. macOS ZIPs contain
an application bundle, not a DMG installer. Windows package CI records the MSVC
DLL import list; it does not bundle the Visual C++ runtime installer or prove a
fresh Windows installation has every required runtime. A compatible graphics
driver and the applicable Microsoft Visual C++ runtime remain host prerequisites.
macOS staging rejects non-system linked libraries rather than accidentally
shipping an app that depends on a developer's Homebrew installation.

`packaging/config.toml` owns package identity, maintainer, license, distro versions,
minimum macOS version, and the Windows timestamp endpoint. Workflow matrices name
the corresponding runners/container images. Application version comes from Cargo
metadata; `Cargo.lock` remains authoritative. Changing a distro baseline requires
updating both config and the matching workflow image and passing install checks.

## CI behavior

- **Diir checks** preserves format, core tests, all-target checks, Clippy, and debug
  link checks on Linux, macOS, and Windows. It also runs Python packaging tests.
- **Native packages** builds unsigned optimized artifacts for PRs, pushes to
  `main`, and manual dispatches. It does not receive signing secrets.
- Linux package jobs build in their named distro, retain native shared-library
  dependency generation, then install and uninstall in a clean matching container.
  They inspect executable ownership/mode, desktop metadata, and `ldd` resolution.
- **Signed desktop packages** is a separate manual workflow on `main`, or a push
  of a `v` tag matching Cargo's version. It checks that the commit is reachable
  from `main` before signing. It does not create tags or publish a release.
- Every uploaded artifact has a SHA-256 sidecar and JSON manifest with version,
  target, architecture, source commit, baseline, and signed/unsigned status.

Missing signing credentials fail the requested signed job. They never cause a
signed job to silently upload unsigned files. Credential-free package CI remains
usable while certificates are being arranged.

## Configure signing

Before adding certificates, create a GitHub Actions environment named
**release-signing** with deployment branch/tag restrictions permitting only `main`
and release tags, and configure a required human reviewer. The reviewer must
approve only workflow code and a source commit already reviewed on `main`.
Branch-editable workflow checks alone are not a secret-access boundary: an altered
workflow could omit its own trust checks. Tag names alone do not establish that a
commit is trusted. Missing environment restrictions or human approval is a signing
setup blocker; do not store certificates until those controls are configured.

Store the following as environment secrets. Do not commit keys or paste them into
issues, pull requests, or chat.

### macOS

| Secret | Exact value required |
| --- | --- |
| `MACOS_CERTIFICATE_P12_BASE64` | Base64 of an exported **Developer ID Application** `.p12`, including its private key |
| `MACOS_CERTIFICATE_PASSWORD` | Password used when exporting that `.p12` |
| `MACOS_SIGN_IDENTITY` | Full certificate identity, e.g. the `Developer ID Application: … (TEAMID)` entry from `security find-identity -v -p codesigning` |
| `APPLE_TEAM_ID` | Apple Developer Program Team ID matching the certificate |
| `APPLE_ID` | Apple ID authorized to submit notarization for that team |
| `APPLE_APP_SPECIFIC_PASSWORD` | App-specific password for that Apple ID's notarization submissions |

Use `base64 -i certificate.p12 | pbcopy` on your Mac to copy the encoded material
for the GitHub secret. Ordinary ASCII whitespace from line wrapping or the command's
terminating newline is accepted; other invalid Base64 characters are rejected.
Certificate passwords are used exactly as supplied. The bundle identifier is
`io.github.mfuon2.diir`.

The workflow creates an ephemeral keychain, imports only for signing, signs with
hardened runtime and a secure timestamp, checks the expected authority/team,
requires Apple notarization to be accepted, staples, and verifies the result. The
keychain and temporary credential files are removed on exit. The package manifest
is finalized before signing, so archiving does not invalidate sealed app resources.

### Windows

| Secret | Exact value required |
| --- | --- |
| `WINDOWS_CERTIFICATE_PFX_BASE64` | Base64 of an existing exportable code-signing `.pfx`, including its private key |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password protecting that `.pfx` |
| `WINDOWS_SIGNER_THUMBPRINT` | Exact signing certificate thumbprint, without spaces |

This integration supports an existing PFX. If your certificate is hardware-backed
or you use a cloud signing service, provide the provider's name so that provider
can be integrated; do not export or fabricate a PFX. Certificate procurement and
paid signing-service setup are outside CI.

The script imports into the ephemeral runner's current-user store, selects the
expected certificate, signs with SHA-256 and an RFC3161 timestamp, verifies native
Authenticode policy and the expected signer, then removes imported credentials.
The timestamp endpoint is configurable in `packaging/config.toml`.

## Local packaging

Build on the intended OS/architecture first. For example, on Apple Silicon:

```sh
cargo build --release --locked -p diir-app --target aarch64-apple-darwin
python3 scripts/package.py stage --platform macos --architecture arm64 \
  --binary target/aarch64-apple-darwin/release/diir
python3 scripts/package.py archive --stage dist/stage/macos-arm64 --signing unsigned
```

Stages refuse to overwrite existing directories. Use a fresh `--output` directory
for another build. The packager rejects binaries with the wrong executable format
or architecture. Use full Xcode as documented in `development.md` on macOS.

For Linux, use the exact distro containers/commands in `package.yml`:
`build-linux.sh` builds and packages, and `check-linux-package.sh` validates a clean
installation. Debian shared dependencies come from `dpkg-shlibdeps`; RPM automatic
ELF requirements remain enabled. Explicit runtime-loaded backend dependencies are
listed in package config, because ELF link analysis cannot discover `dlopen` loads.

Install downloaded packages with:

```sh
sudo apt install ./diir_*.deb
# Fedora 45:
sudo dnf install ./diir-*.rpm
```

Each payload includes the project Apache-2.0 license and third-party notices
collected from Cargo's target-filtered package metadata and dependency license
files. Dependency license expressions/texts retain their own terms; the project
license does not relicense those dependencies.

## Validation limits

See `validation.md` for results actually observed. A configured signing workflow
is not evidence of a signed/notarized build. Real certificates and successful
native signature verification are required before claiming signing acceptance.
