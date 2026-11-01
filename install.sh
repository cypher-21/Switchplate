#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${HOME}/.local/share/applications"
AUTOSTART_DIR="${HOME}/.config/autostart"
ICON_DIR="${HOME}/.local/share/icons/hicolor/128x128/apps"
PIXMAPS_DIR="${HOME}/.local/share/pixmaps"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="${PROJECT_DIR}/src-tauri/target/release/opencode-notch"

if [ -f "${BIN_PATH}" ]; then
  echo "Found prebuilt release binary: ${BIN_PATH}"
else
  echo "Release binary not found. Building OpenCode Notch frontend assets..."
  cd "${PROJECT_DIR}"
  if [ ! -d "${PROJECT_DIR}/node_modules" ]; then
    npm install
  fi
  npm run build

  echo "Building OpenCode Notch release binary with cargo..."
  cd "${PROJECT_DIR}/src-tauri"
  cargo build --release
  strip "${BIN_PATH}"
fi

echo "Installing binary and icons..."
mkdir -p "${INSTALL_DIR}"
mkdir -p "${DESKTOP_DIR}"
mkdir -p "${AUTOSTART_DIR}"
mkdir -p "${ICON_DIR}"
mkdir -p "${PIXMAPS_DIR}"

cp -f "${BIN_PATH}" "${INSTALL_DIR}/opencode-notch.tmp"
mv -f "${INSTALL_DIR}/opencode-notch.tmp" "${INSTALL_DIR}/opencode-notch"
chmod +x "${INSTALL_DIR}/opencode-notch"

# Install icons to standard XDG directories
cp -f "${PROJECT_DIR}/src-tauri/icons/128x128.png" "${ICON_DIR}/opencode-notch.png"
cp -f "${PROJECT_DIR}/src-tauri/icons/128x128.png" "${PIXMAPS_DIR}/opencode-notch.png"

cat <<EOF > "${DESKTOP_DIR}/opencode-notch.desktop"
[Desktop Entry]
Name=OpenCode Notch
GenericName=OpenCode Approval Notch
Comment=Floating top notification panel for OpenCode CLI approvals
Exec=${INSTALL_DIR}/opencode-notch
Icon=opencode-notch
Terminal=false
Type=Application
Categories=Development;Utility;
StartupNotify=true
StartupWMClass=opencode-notch
EOF
chmod +x "${DESKTOP_DIR}/opencode-notch.desktop"

# Remove duplicate XDG autostart file to ensure only systemd manages the daemon
rm -f "${AUTOSTART_DIR}/opencode-notch.desktop"

# Update desktop and icon databases
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi

SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
mkdir -p "${SYSTEMD_USER_DIR}"

# Install and enable systemd user service
cat <<EOF > "${SYSTEMD_USER_DIR}/opencode-notch.service"
[Unit]
Description=OpenCode Notch Approval Utility
After=graphical-session.target

[Service]
ExecStart=${INSTALL_DIR}/opencode-notch
Restart=always
RestartSec=3
Environment=DISPLAY=:0

[Install]
WantedBy=default.target
EOF

if command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload || true
  systemctl --user enable opencode-notch.service || true
  echo "Starting background service opencode-notch.service..."
  systemctl --user restart opencode-notch.service || true
fi

echo "Installation complete."
echo "Binary: ${INSTALL_DIR}/opencode-notch"
echo "Desktop launcher: ${DESKTOP_DIR}/opencode-notch.desktop"
echo "Icon: ${ICON_DIR}/opencode-notch.png"
