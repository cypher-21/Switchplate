#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${HOME}/.local/share/applications"
AUTOSTART_DIR="${HOME}/.config/autostart"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="${PROJECT_DIR}/src-tauri/target/release/opencode-notch"

echo "Building OpenCode Notch frontend..."
cd "${PROJECT_DIR}"
npm run build

echo "Building OpenCode Notch release binary..."
cd "${PROJECT_DIR}/src-tauri"
cargo build --release
strip "${BIN_PATH}"

mkdir -p "${INSTALL_DIR}"
mkdir -p "${DESKTOP_DIR}"
mkdir -p "${AUTOSTART_DIR}"

echo "Installing binary to ${INSTALL_DIR}/opencode-notch..."
cp -f "${BIN_PATH}" "${INSTALL_DIR}/opencode-notch"
chmod +x "${INSTALL_DIR}/opencode-notch"

cat <<EOF > "${DESKTOP_DIR}/opencode-notch.desktop"
[Desktop Entry]
Name=OpenCode Notch
Comment=Floating top notification panel for OpenCode CLI permissions
Exec=${INSTALL_DIR}/opencode-notch
Icon=${PROJECT_DIR}/src-tauri/icons/128x128.png
Terminal=false
Type=Application
Categories=Utility;Development;
EOF
chmod +x "${DESKTOP_DIR}/opencode-notch.desktop"

# Copy to autostart
cp -f "${DESKTOP_DIR}/opencode-notch.desktop" "${AUTOSTART_DIR}/opencode-notch.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true
fi

echo "Installation complete."
echo "Binary: ${INSTALL_DIR}/opencode-notch"
echo "Desktop launcher: ${DESKTOP_DIR}/opencode-notch.desktop"
echo "Autostart entry: ${AUTOSTART_DIR}/opencode-notch.desktop"
