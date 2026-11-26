#!/usr/bin/env bash
set -euo pipefail
platform=${1:?Pass debian or fedora}
case "$platform" in
  debian)
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y build-essential clang cmake pkg-config libasound2-dev libfontconfig1-dev \
      libfreetype6-dev libwayland-dev libx11-dev libx11-xcb-dev libxcb1-dev libxkbcommon-dev \
      libxkbcommon-x11-dev libssl-dev libzstd-dev libvulkan-dev curl ca-certificates git \
      python3 dpkg-dev desktop-file-utils
    ;;
  fedora)
    dnf install -y gcc gcc-c++ clang cmake pkgconf-pkg-config alsa-lib-devel fontconfig-devel \
      freetype-devel wayland-devel libX11-devel libxcb-devel libxkbcommon-devel \
      libxkbcommon-x11-devel openssl-devel libzstd-devel vulkan-loader-devel curl \
      ca-certificates git python3 rpm-build desktop-file-utils
    ;;
  *) echo 'Unsupported package platform' >&2; exit 1 ;;
esac
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/diir-rustup-init.sh
sh /tmp/diir-rustup-init.sh -y --profile minimal --default-toolchain none
export PATH="$HOME/.cargo/bin:$PATH"
rustup show
cargo build --release --locked -p diir-app --target x86_64-unknown-linux-gnu
desktop-file-validate packaging/linux/diir.desktop
python3 scripts/package.py stage --platform "$platform" --architecture x86_64 \
  --binary target/x86_64-unknown-linux-gnu/release/diir
python3 scripts/package.py linux --stage "dist/stage/$platform-x86_64"
