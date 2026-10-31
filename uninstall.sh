#!/usr/bin/env bash
set -euo pipefail

echo "Stopping and disabling background service..."
if command -v systemctl >/dev/null 2>&1; then
  systemctl --user stop opencode-notch.service 2>/dev/null || true
  systemctl --user disable opencode-notch.service 2>/dev/null || true
  rm -f "${HOME}/.config/systemd/user/opencode-notch.service"
  rm -f "${HOME}/.config/systemd/user/default.target.wants/opencode-notch.service"
  systemctl --user daemon-reload 2>/dev/null || true
fi

echo "Terminating any lingering processes..."
pkill -9 -f opencode-notch 2>/dev/null || true

echo "Removing installed binary..."
rm -f "${HOME}/.local/bin/opencode-notch"
rm -f "${HOME}/.local/bin/opencode-notch.tmp"

echo "Removing desktop entries and autostart..."
rm -f "${HOME}/.local/share/applications/opencode-notch.desktop"
rm -f "${HOME}/.config/autostart/opencode-notch.desktop"

echo "Removing application icons..."
rm -f "${HOME}/.local/share/icons/hicolor/128x128/apps/opencode-notch.png"
rm -f "${HOME}/.local/share/pixmaps/opencode-notch.png"

echo "Removing webview cache..."
rm -rf "${HOME}/.local/share/dev.opencode.notch"
rm -rf "${HOME}/.local/share/dev.switchplate.desktop"

echo "Updating desktop and icon caches..."
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "Uninstall complete. All old versions and components removed."

