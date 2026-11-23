#!/bin/sh
# Build ApexDiag for a real iPad/iPhone and install it over USB (or Wi-Fi once
# paired). Run from the repo root:   ./scripts/ios-install.sh
#
# First time only, on the iPad: plug it into the Mac, tap "Trust", and turn on
# Settings > Privacy & Security > Developer Mode (the iPad restarts).
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APPLE="$ROOT/src-tauri/gen/apple"
OUT="$ROOT/target/xcode-dev"
APP="$OUT/Build/Products/release-iphoneos/ApexDiag.app"
BUNDLE=com.drny85.apexdiag

export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:/opt/homebrew/bin:$PATH"

echo "==> Looking for a connected iPad or iPhone"
LIST="$(mktemp)"
xcrun devicectl list devices --json-output "$LIST" >/dev/null 2>&1 || true
# Pick the first paired physical iPad, or the device in $DEVICE_UDID.
DEVICE="${DEVICE_UDID:+$DEVICE_UDID	device}"
[ -n "$DEVICE" ] || DEVICE="$(/usr/bin/python3 - "$LIST" <<'PY'
import json, sys
try:
    devs = json.load(open(sys.argv[1]))["result"]["devices"]
except Exception:
    devs = []
def ok(d):
    hw = d.get("hardwareProperties", {})
    return hw.get("platform") == "iOS" and hw.get("reality") == "physical" \
        and d.get("connectionProperties", {}).get("pairingState") == "paired"
ipads = [d for d in devs if ok(d) and d["hardwareProperties"].get("deviceType") == "iPad"]
any_ios = [d for d in devs if ok(d)]
pick = (ipads or [None])[0]
if pick:
    print(pick["hardwareProperties"]["udid"] + "\t" + pick["deviceProperties"]["name"])
PY
)"
rm -f "$LIST"
if [ -z "$DEVICE" ]; then
  echo "No paired iPad found. Plug it in with a cable, unlock it, tap Trust, then run this again." >&2
  exit 1
fi
UDID="$(printf '%s' "$DEVICE" | cut -f1)"
NAME="$(printf '%s' "$DEVICE" | cut -f2)"
echo "    $NAME ($UDID)"

echo "==> Building (first build takes a few minutes)"
cd "$APPLE"
xcodebuild -project apexdiag.xcodeproj -scheme apexdiag_iOS -configuration release \
  -sdk iphoneos -destination "id=$UDID" -derivedDataPath "$OUT" \
  -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
  build | grep -E "error:|warning: .*provision|BUILD (SUCCEEDED|FAILED)" || true
[ -d "$APP" ] || { echo "Build failed. Open src-tauri/gen/apple/apexdiag.xcodeproj in Xcode to see why." >&2; exit 1; }

echo "==> Installing on $NAME"
xcrun devicectl device install app --device "$UDID" "$APP"
echo "==> Launching"
xcrun devicectl device process launch --device "$UDID" "$BUNDLE" || \
  echo "Installed. If it will not open, trust the developer in Settings > General > VPN & Device Management."
echo "Done. ApexDiag is on $NAME."
