#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${HOME}/.local/share/applications"
AUTOSTART_DIR="${HOME}/.config/autostart"
ICON_DIR="${HOME}/.local/share/icons/hicolor/128x128/apps"
PIXMAPS_DIR="${HOME}/.local/share/pixmaps"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="${PROJECT_DIR}/src-tauri/target/release/switchplate"

# Fallback check if compiled under legacy name
if [ ! -f "${BIN_PATH}" ] && [ -f "${PROJECT_DIR}/src-tauri/target/release/opencode-notch" ]; then
  BIN_PATH="${PROJECT_DIR}/src-tauri/target/release/opencode-notch"
fi

if [ -f "${BIN_PATH}" ]; then
  echo "Found prebuilt release binary: ${BIN_PATH}"
else
  echo "Release binary not found. Building Switchplate frontend assets..."
  cd "${PROJECT_DIR}"
  if [ ! -d "${PROJECT_DIR}/node_modules" ]; then
    npm install
  fi
  npm run build

  echo "Building Switchplate release binary with cargo..."
  cd "${PROJECT_DIR}/src-tauri"
  cargo build --release
  BIN_PATH="${PROJECT_DIR}/src-tauri/target/release/switchplate"
  strip "${BIN_PATH}"
fi

echo "Installing binary and icons..."
mkdir -p "${INSTALL_DIR}"
mkdir -p "${DESKTOP_DIR}"
mkdir -p "${AUTOSTART_DIR}"
mkdir -p "${ICON_DIR}"
mkdir -p "${PIXMAPS_DIR}"

cp -f "${BIN_PATH}" "${INSTALL_DIR}/switchplate.tmp"
mv -f "${INSTALL_DIR}/switchplate.tmp" "${INSTALL_DIR}/switchplate"
chmod +x "${INSTALL_DIR}/switchplate"

# Maintain backwards compatibility symlink for legacy commands
ln -sf "${INSTALL_DIR}/switchplate" "${INSTALL_DIR}/opencode-notch"

# Install icons to standard XDG directories
cp -f "${PROJECT_DIR}/src-tauri/icons/128x128.png" "${ICON_DIR}/switchplate.png"
cp -f "${PROJECT_DIR}/src-tauri/icons/128x128.png" "${PIXMAPS_DIR}/switchplate.png"
ln -sf "${ICON_DIR}/switchplate.png" "${ICON_DIR}/opencode-notch.png"
ln -sf "${PIXMAPS_DIR}/switchplate.png" "${PIXMAPS_DIR}/opencode-notch.png"

# Clean legacy desktop files
rm -f "${DESKTOP_DIR}/opencode-notch.desktop"
rm -f "${AUTOSTART_DIR}/opencode-notch.desktop"
rm -f "${AUTOSTART_DIR}/switchplate.desktop"

cat <<EOF > "${DESKTOP_DIR}/switchplate.desktop"
[Desktop Entry]
Name=Switchplate
GenericName=Universal AI Control Surface
Comment=Precision floating control surface and approval island for AI coding agents
Exec=${INSTALL_DIR}/switchplate
Icon=switchplate
Terminal=false
Type=Application
Categories=Development;Utility;
StartupNotify=true
StartupWMClass=switchplate
EOF
chmod +x "${DESKTOP_DIR}/switchplate.desktop"

# Update desktop and icon databases
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi

# Clean legacy systemd unit
if command -v systemctl >/dev/null 2>&1; then
  systemctl --user stop opencode-notch.service 2>/dev/null || true
  systemctl --user disable opencode-notch.service 2>/dev/null || true
  rm -f "${HOME}/.config/systemd/user/opencode-notch.service"
  rm -f "${HOME}/.config/systemd/user/default.target.wants/opencode-notch.service"
fi

SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
mkdir -p "${SYSTEMD_USER_DIR}"

# Install and enable modern systemd user service
cat <<EOF > "${SYSTEMD_USER_DIR}/switchplate.service"
[Unit]
Description=Switchplate - Universal AI Control Surface & Approval Island
After=graphical-session.target

[Service]
ExecStart=${INSTALL_DIR}/switchplate
Restart=on-failure
RestartSec=3
Environment=DISPLAY=:0

[Install]
WantedBy=default.target
EOF

if command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload || true
  systemctl --user enable switchplate.service || true
  echo "Starting background service switchplate.service..."
  systemctl --user restart switchplate.service || true
fi

echo "Installation complete."
echo "Binary: ${INSTALL_DIR}/switchplate"
echo "Desktop launcher: ${DESKTOP_DIR}/switchplate.desktop"
echo "Icon: ${ICON_DIR}/switchplate.png"
echo "Service: switchplate.service"
