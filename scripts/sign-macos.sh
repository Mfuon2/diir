#!/usr/bin/env bash
set -euo pipefail

: "${MACOS_CERTIFICATE_P12_BASE64:?Set the Developer ID Application P12 secret}"
: "${MACOS_CERTIFICATE_PASSWORD:?Required signing configuration is missing}"
: "${MACOS_SIGN_IDENTITY:?Set the full Developer ID Application signing identity}"
: "${APPLE_TEAM_ID:?Set the Apple Developer Team ID}"
: "${APPLE_ID:?Set the Apple ID used for notarization}"
: "${APPLE_APP_SPECIFIC_PASSWORD:?Required notarization configuration is missing}"
: "${RUNNER_TEMP:?Run in the signing CI job}"

bundle=${1:?Pass the staged app bundle}
[[ -d "$bundle" && "$MACOS_SIGN_IDENTITY" == 'Developer ID Application:'* ]] || {
  echo 'Expected an app bundle and Developer ID Application identity' >&2
  exit 1
}
signing_dir=$(mktemp -d "$RUNNER_TEMP/diir-sign.XXXXXX")
keychain="$signing_dir/signing.keychain-db"
keychain_password=$(openssl rand -hex 32)
cleanup() {
  security delete-keychain "$keychain" >/dev/null 2>&1 || true
  rm -rf "$signing_dir"
}
trap cleanup EXIT
export DIIR_P12_PATH="$signing_dir/developer.p12"
python3 - <<'PY'
import os
from pathlib import Path
from scripts.package import decode_certificate
path = Path(os.environ['DIIR_P12_PATH'])
path.write_bytes(decode_certificate(os.environ['MACOS_CERTIFICATE_P12_BASE64']))
path.chmod(0o600)
PY
security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
security import "$DIIR_P12_PATH" -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign -T /usr/bin/security -t cert -f pkcs12 -k "$keychain"
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$keychain_password" "$keychain" >/dev/null
codesign --force --options runtime --timestamp --keychain "$keychain" --sign "$MACOS_SIGN_IDENTITY" "$bundle"
codesign --verify --deep --strict --verbose=2 "$bundle"
codesign --display --verbose=4 "$bundle" 2> "$signing_dir/signature.txt"
grep -Fx "TeamIdentifier=$APPLE_TEAM_ID" "$signing_dir/signature.txt"
grep -Fx "Authority=$MACOS_SIGN_IDENTITY" "$signing_dir/signature.txt"
ditto -c -k --keepParent "$bundle" "$signing_dir/notarize.zip"
xcrun notarytool submit "$signing_dir/notarize.zip" --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_SPECIFIC_PASSWORD" \
  --wait --output-format json > "$signing_dir/notary-result.json"
python3 - "$signing_dir/notary-result.json" <<'PY'
import json
import sys
from pathlib import Path
result = json.loads(Path(sys.argv[1]).read_text())
if result.get('status') != 'Accepted':
    raise SystemExit(f"Apple notarization was not accepted: {result.get('status', 'unknown')}")
PY
xcrun stapler staple "$bundle"
xcrun stapler validate "$bundle"
spctl --assess --type execute --verbose=2 "$bundle"
