#!/usr/bin/env bash
# Installs vterm to /Applications without triggering Gatekeeper's "damaged"
# warning. Browsers tag downloaded files with a quarantine flag that Gatekeeper
# rejects for unsigned apps; curl does not, so this path just works.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/boytur/vterm/master/install.sh | bash
set -euo pipefail

APP_NAME="vterm"
URL="https://github.com/boytur/vterm/releases/latest/download/${APP_NAME}-macos.dmg"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${APP_NAME}..."
curl -fsSL "$URL" -o "$TMP_DIR/${APP_NAME}.dmg"

echo "Installing to /Applications..."
MOUNT_DIR=$(hdiutil attach "$TMP_DIR/${APP_NAME}.dmg" -nobrowse -quiet | awk -F'\t' '/\/Volumes\// {print $NF; exit}')
rm -rf "/Applications/${APP_NAME}.app"
cp -R "$MOUNT_DIR/${APP_NAME}.app" /Applications/
hdiutil detach "$MOUNT_DIR" -quiet

echo "Done. Launch with: open -a ${APP_NAME}"
