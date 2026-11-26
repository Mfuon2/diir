#!/usr/bin/env bash
set -euo pipefail
platform=${1:?Pass debian or fedora}
case "$platform" in
  debian)
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y binutils desktop-file-utils
    packages=(dist/*.deb)
    [[ ${#packages[@]} == 1 && -f ${packages[0]} ]]
    apt-get install -y "/source/${packages[0]}"
    dpkg-query -W diir
    ;;
  fedora)
    dnf install -y binutils desktop-file-utils
    packages=(dist/*.rpm)
    [[ ${#packages[@]} == 1 && -f ${packages[0]} ]]
    dnf install -y "/source/${packages[0]}"
    rpm -q diir
    ;;
  *) echo 'Unsupported package platform' >&2; exit 1 ;;
esac
[[ -x /usr/bin/diir ]]
[[ $(stat -c '%u:%g:%a' /usr/bin/diir) == 0:0:755 ]]
desktop-file-validate /usr/share/applications/diir.desktop
ldd /usr/bin/diir > /tmp/diir-shared-libraries.txt
cat /tmp/diir-shared-libraries.txt
if grep -q 'not found' /tmp/diir-shared-libraries.txt; then exit 1; fi
if [[ "$platform" == debian ]]; then apt-get remove -y diir; else dnf remove -y diir; fi
[[ ! -e /usr/bin/diir ]]
