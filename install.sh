#!/usr/bin/env bash
# Installs vterm to /Applications without triggering Gatekeeper's "damaged"
# warning. Browsers tag downloaded files with a quarantine flag that Gatekeeper
# rejects for unsigned apps; curl does not, so this path just works.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/boytur/vterm/master/install.sh | bash
set -euo pipefail

APP_NAME="vterm"
INSTALL_DIR="/Applications"
CURRENT_APP="$INSTALL_DIR/${APP_NAME}.app"
URL="https://github.com/boytur/vterm/releases/latest/download/${APP_NAME}-macos.dmg"
TMP_DIR="$(mktemp -d)"
MOUNT_DIR=""
cleanup() {
  if [[ -n "$MOUNT_DIR" ]]; then
    hdiutil detach "$MOUNT_DIR" -force >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "Downloading ${APP_NAME}..."
curl -fL --retry 3 --retry-delay 1 -sS "$URL" -o "$TMP_DIR/${APP_NAME}.dmg"

echo "Installing to /Applications..."
MOUNT_DIR=$(hdiutil attach "$TMP_DIR/${APP_NAME}.dmg" -nobrowse -plist |
  grep -A1 '<key>mount-point</key>' | grep string | head -1 | sed -E 's/.*<string>(.*)<\/string>.*/\1/')
SOURCE_APP="$MOUNT_DIR/${APP_NAME}.app"
[[ -x "$SOURCE_APP/Contents/MacOS/$APP_NAME" ]] || {
  echo "Downloaded app bundle is invalid" >&2
  exit 1
}
cp -R "$SOURCE_APP" "$TMP_DIR/${APP_NAME}.app"

# A running old process can keep using the old bundle after installation.
osascript -e "tell application \"${APP_NAME}\" to quit" >/dev/null 2>&1 || true
for _ in {1..20}; do
  pgrep -x "$APP_NAME" >/dev/null || break
  sleep 0.25
done
if pgrep -x "$APP_NAME" >/dev/null; then
  echo "Could not stop the running ${APP_NAME}; close it and try again." >&2
  exit 1
fi

BACKUP_APP="$TMP_DIR/${APP_NAME}.app.old"
if [[ -d "$CURRENT_APP" ]]; then
  mv "$CURRENT_APP" "$BACKUP_APP"
fi
if ! mv "$TMP_DIR/${APP_NAME}.app" "$CURRENT_APP"; then
  [[ -d "$BACKUP_APP" ]] && mv "$BACKUP_APP" "$CURRENT_APP"
  exit 1
fi
xattr -dr com.apple.quarantine "$CURRENT_APP" 2>/dev/null || true
VERSION=$(plutil -extract CFBundleShortVersionString raw -o - "$CURRENT_APP/Contents/Info.plist" 2>/dev/null || true)

echo "Done. Installed v${VERSION:-unknown}. Launch with: open -a ${APP_NAME}"
