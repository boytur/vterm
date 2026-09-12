#!/usr/bin/env bash
# Installs vterm without triggering security or quarantine warnings.
#
# Supported platforms:
#   - macOS: downloads DMG and installs to /Applications/vterm.app
#   - Windows: downloads ZIP, installs to %LOCALAPPDATA%\Programs\vterm,
#              adds to User PATH, and creates a Start Menu shortcut.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/boytur/vterm/master/install.sh | bash
set -euo pipefail

APP_NAME="vterm"
OS="$(uname -s 2>/dev/null || echo unknown)"
IS_WINDOWS=false
IS_MACOS=false
IS_WSL=false

case "$OS" in
  Darwin*)
    IS_MACOS=true
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT*)
    IS_WINDOWS=true
    ;;
  Linux*)
    if grep -qi microsoft /proc/version 2>/dev/null; then
      IS_WINDOWS=true
      IS_WSL=true
    else
      echo "Linux is not yet supported by vterm." >&2
      exit 1
    fi
    ;;
  *)
    echo "Unsupported operating system: $OS" >&2
    exit 1
    ;;
esac

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t vterm)"
MOUNT_DIR=""

cleanup() {
  if [[ -n "$MOUNT_DIR" ]]; then
    hdiutil detach "$MOUNT_DIR" -force >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if [[ "$IS_MACOS" == "true" ]]; then
  INSTALL_DIR="/Applications"
  CURRENT_APP="$INSTALL_DIR/${APP_NAME}.app"
  URL="https://github.com/boytur/vterm/releases/latest/download/${APP_NAME}-macos.dmg"

  echo "Downloading ${APP_NAME} for macOS..."
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

  # Swap paths instead of quitting vterm. macOS keeps the running process on the
  # old bundle while new launches use the replacement at /Applications/vterm.app.
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

  echo "Done. Installed v${VERSION:-unknown}. Relaunch vterm to use the update: open -a ${APP_NAME}"

elif [[ "$IS_WINDOWS" == "true" ]]; then
  URL="https://github.com/boytur/vterm/releases/latest/download/${APP_NAME}-windows.zip"

  # Resolve target directory on Windows (%LOCALAPPDATA%\Programs\vterm)
  if command -v cygpath >/dev/null 2>&1; then
    WIN_LOCALAPPDATA="${LOCALAPPDATA:-$USERPROFILE\\AppData\\Local}"
    INSTALL_DIR="$(cygpath -u "$WIN_LOCALAPPDATA")/Programs/${APP_NAME}"
    WIN_INSTALL_DIR="$(cygpath -w "$INSTALL_DIR")"
  elif [[ "$IS_WSL" == "true" ]] && command -v wslpath >/dev/null 2>&1; then
    WIN_LOCALAPPDATA="$(powershell.exe -NoProfile -Command '[Environment]::GetFolderPath("LocalApplicationData")' 2>/dev/null | tr -d '\r')"
    INSTALL_DIR="$(wslpath -u "$WIN_LOCALAPPDATA")/Programs/${APP_NAME}"
    WIN_INSTALL_DIR="$WIN_LOCALAPPDATA\\Programs\\${APP_NAME}"
  else
    WIN_LOCALAPPDATA="${LOCALAPPDATA:-${USERPROFILE:-$HOME/AppData/Local}}"
    INSTALL_DIR="${HOME}/AppData/Local/Programs/${APP_NAME}"
    WIN_INSTALL_DIR="${INSTALL_DIR//\//\\}"
  fi

  echo "Downloading ${APP_NAME} for Windows..."
  curl -fL --retry 3 --retry-delay 1 -sS "$URL" -o "$TMP_DIR/${APP_NAME}.zip"

  echo "Extracting ${APP_NAME}..."
  EXTRACT_DIR="$TMP_DIR/extracted"
  mkdir -p "$EXTRACT_DIR"

  if command -v unzip >/dev/null 2>&1; then
    unzip -q -o "$TMP_DIR/${APP_NAME}.zip" -d "$EXTRACT_DIR"
  elif command -v tar >/dev/null 2>&1; then
    tar -xf "$TMP_DIR/${APP_NAME}.zip" -C "$EXTRACT_DIR"
  elif command -v powershell.exe >/dev/null 2>&1; then
    if command -v cygpath >/dev/null 2>&1; then
      WIN_ZIP="$(cygpath -w "$TMP_DIR/${APP_NAME}.zip")"
      WIN_EXTRACT="$(cygpath -w "$EXTRACT_DIR")"
    elif [[ "$IS_WSL" == "true" ]] && command -v wslpath >/dev/null 2>&1; then
      WIN_ZIP="$(wslpath -w "$TMP_DIR/${APP_NAME}.zip")"
      WIN_EXTRACT="$(wslpath -w "$EXTRACT_DIR")"
    else
      WIN_ZIP="$TMP_DIR/${APP_NAME}.zip"
      WIN_EXTRACT="$EXTRACT_DIR"
    fi
    powershell.exe -NoProfile -Command "Expand-Archive -Path '$WIN_ZIP' -DestinationPath '$WIN_EXTRACT' -Force"
  else
    echo "Error: unzip, tar, or powershell.exe is required to extract the archive." >&2
    exit 1
  fi

  SOURCE_EXE="$(find "$EXTRACT_DIR" -type f -name "${APP_NAME}.exe" | head -1)"
  if [[ -z "$SOURCE_EXE" || ! -f "$SOURCE_EXE" ]]; then
    echo "Error: Downloaded archive does not contain ${APP_NAME}.exe" >&2
    exit 1
  fi

  echo "Installing to ${WIN_INSTALL_DIR}..."
  mkdir -p "$INSTALL_DIR"
  TARGET_EXE="$INSTALL_DIR/${APP_NAME}.exe"
  BACKUP_EXE="$INSTALL_DIR/${APP_NAME}.exe.old"

  # Swap executable in case vterm is currently running
  if [[ -f "$TARGET_EXE" ]]; then
    mv -f "$TARGET_EXE" "$BACKUP_EXE" 2>/dev/null || true
  fi

  cp -f "$SOURCE_EXE" "$TARGET_EXE"
  chmod +x "$TARGET_EXE" 2>/dev/null || true
  rm -f "$BACKUP_EXE" 2>/dev/null || true

  # Clear Windows Mark of the Web quarantine, add to User PATH, and create Start Menu shortcut
  if command -v powershell.exe >/dev/null 2>&1; then
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "
      \$dir = '$WIN_INSTALL_DIR'
      \$exe = Join-Path \$dir '${APP_NAME}.exe'
      if (Test-Path \$exe) {
        Unblock-File -Path \$exe -ErrorAction SilentlyContinue
      }

      \$p = [Environment]::GetEnvironmentVariable('Path', 'User')
      if ((\$p -split ';') -notcontains \$dir) {
        [Environment]::SetEnvironmentVariable('Path', (\$p.TrimEnd(';') + ';' + \$dir), 'User')
      }

      \$ws = New-Object -ComObject WScript.Shell
      \$startMenu = [Environment]::GetFolderPath('StartMenu')
      \$shortcutDir = Join-Path \$startMenu 'Programs'
      if (Test-Path \$shortcutDir) {
        \$shortcut = \$ws.CreateShortcut((Join-Path \$shortcutDir '${APP_NAME}.lnk'))
        \$shortcut.TargetPath = \$exe
        \$shortcut.IconLocation = \$exe + ',0'
        \$shortcut.WorkingDirectory = [Environment]::GetFolderPath('UserProfile')
        \$shortcut.Description = 'vterm terminal emulator'
        \$shortcut.Save()
      }
    " >/dev/null 2>&1 || true
  fi


  echo "Done. Installed ${APP_NAME} to: ${WIN_INSTALL_DIR}\\${APP_NAME}.exe"
  echo "Start Menu shortcut created: ${APP_NAME}"
  echo "To launch from terminal, open a new terminal window and type: ${APP_NAME}"
fi

