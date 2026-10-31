# OpenCode Notch

A lightweight, Linux-native floating notification panel anchored at the top-center of your screen that serves as an instant approval and denial interface for OpenCode CLI.

No mascots, no heavy electron processes, no multi-agent relay hooks. Built specifically and purely for OpenCode CLI.

---

## Features

- **Top-Center Floating Notch:** Discreet 10px status dot when idle (green for connected, red for disconnected, yellow for connecting).
- **Instant Expand on Permission Request:** When OpenCode CLI halts on a tool action (`bash`, `edit`, `webfetch`), the notch smoothly expands into an approval card.
- **Single-Click & Global Shortcut Decisions:**
  - **Accept / Once:** `Ctrl+Shift+A` (or click Accept)
  - **Reject / Deny:** `Ctrl+Shift+D` (or click Reject)
  - **Always Allow:** `Ctrl+Shift+W` (or click Always)
  - **Toggle Notch Visibility:** `Ctrl+Shift+N`
- **Native Auto-Discovery:** Automatically detects running OpenCode service instances from `~/.local/state/opencode/service.json` and `~/.config/opencode/service.json` without requiring manual port or password configuration.
- **Native OpenCode v2 Protocol:** Integrates directly with OpenCode v2's `/api/event` SSE stream and `/api/session/{sessionID}/permission/{requestID}/reply` endpoint.
- **Ultra Lightweight:** 10 MB stripped binary built with Tauri 2 and Solid.js. Uses approximately 25-30 MB RAM.

---

## Requirements

- Linux (X11 or Wayland)
- WebKit2GTK 4.1 (`libwebkit2gtk-4.1-dev` on Debian/Ubuntu)
- OpenCode CLI (`opencode`)

---

## Building from Source

```bash
# 1. Install frontend dependencies and build assets
npm install
npm run build

# 2. Build Rust release binary
cd src-tauri
cargo build --release
strip target/release/opencode-notch
```

---

## Installation

Run the provided installation script:

```bash
./install.sh
```

This installs:
- Executable to `~/.local/bin/opencode-notch`
- Desktop entry to `~/.local/share/applications/opencode-notch.desktop`

---

## Usage

Start the notch in the background:

```bash
opencode-notch &
```

Or launch it via your desktop environment's application menu.

When you run OpenCode commands in your terminal:

```bash
opencode
# or
opencode run "inspect git status and run tests"
```

Whenever OpenCode requests permission for bash commands or file modifications, the notch will drop down from the top center showing:
1. Tool badge (`BASH`, `EDIT`, etc.)
2. Exact command / file parameters
3. `Accept`, `Reject`, and `Always` action buttons
4. Queue indicator if multiple permissions are waiting

You can click any button or press the keyboard shortcuts (`Ctrl+Shift+A`, `Ctrl+Shift+D`, `Ctrl+Shift+W`) without leaving your terminal.

---

## Configuration

Configuration is optional. The notch automatically detects the running OpenCode service. To customize settings, create `~/.config/opencode-notch/config.toml`:

```toml
[opencode]
server_url = "http://localhost:4096"
password_env = "OPENCODE_API_PASSWORD"

[appearance]
notch_width = 400
notch_height = 220
collapsed_size = 32

[behavior]
auto_hide_seconds = 0
sound_enabled = true
```

---

## Architecture Overview

```
OpenCode CLI (v2.2+)
  |-- Service daemon (~/.local/state/opencode/service.json)
  |-- SSE Stream: GET /api/event (emits permission.asked, permission.replied)
  \-- Permission API: POST /api/session/{sessionID}/permission/{requestID}/reply

opencode-notch (Rust backend + Solid.js)
  |-- src-tauri/src/opencode/discovery.rs (detects port and password)
  |-- src-tauri/src/opencode/client.rs (SSE listener and reply dispatcher)
  |-- src-tauri/src/opencode/events.rs (flexible v2 event parser)
  |-- src-tauri/src/state.rs (NotchState queue and connection status)
  |-- src-tauri/src/window.rs (top-center screen positioning and state resizing)
  \-- src/ (Solid.js reactive UI: StatusDot, ApprovalCard, ActionButtons)
```

---

## Future Phase: Local Voice Dictation

Phase 2 will integrate an embedded local voice model (`whisper.cpp` tiny.en, ~75MB) and microphone capture (`cpal`) to allow holding a hotkey and dictating natural language instructions directly into active OpenCode sessions.

