# Switchplate

**Precision Dynamic Island & Control Surface for AI Coding Agents**

Switchplate is a Linux-native desktop companion that brings a floating dynamic island and precision hardware-inspired control surface to agentic coding runtimes—including OpenCode, Claude Code, Gemini CLI, Antigravity, and local LLM agents.

When autonomous AI agents request tool approval (shell commands, file modifications, network fetches), Switchplate surfaces an unobtrusive top-anchored approval card. Make decisions instantly using mouse controls or global keyboard shortcuts from anywhere in your desktop without losing terminal focus.

---

## Key Features

- **Industrial Swiss Interface:** Designed as a tactile hardware instrument. High contrast, warm brass accents (#B58B53), solid matte charcoal (#171715), and IBM Plex typography. No emojis, no glass blur, no distractions.
- **Dynamic Top Island:** Floats discreetly below your top bar. Auto-hides when idle, and expands instantly when tool approval is required.
- **Immediate Response Lifecycle:** One-click instant execution with no artificial delays.
- **Acoustic Studio Chimes:** Modeled harmonic acoustic bells synthesized using the Web Audio API with zero external media files. Distinct cues for incoming requests and task completion.
- **Response Ready Popups:** Informs you immediately when an AI agent finishes generating code or completes execution, allowing you to review results in your terminal.
- **Global Keyboard Shortcuts:**
  - `Ctrl + Shift + A`: Accept / Allow Once
  - `Ctrl + Shift + D`: Reject / Deny
  - `Ctrl + Shift + W`: Always Allow (Remember Rule)
  - `Ctrl + Shift + O`: Open / Toggle Switchplate Island
  - `Ctrl + Shift + S`: Open Dedicated Settings Surface
  - `Escape`: Dismiss / Hide Island or Settings
- **System Tray Integration:** Real-time connection diagnostics (`Connected` / `Connecting` / `Offline`), single-instance process protection, and instant access to controls.
- **Lightweight Native Performance:** Built with Tauri 2 and Solid.js. 14 MB stripped binary with minimal RAM footprint (~28 MB).
- **Universal Multi-Agent Ready:** Architecture decoupled from single tools, designed to integrate with OpenCode, Claude, Gemini, and Antigravity.

---

## Visual Architecture

```
+-----------------------------------------------------------------------+
|  [ = SWITCHPLATE | CONNECTED ]                                        |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| | OPENCODE REQUEST                                           [SHELL]  |
|   ACTIONS REQUIRING YOUR APPROVAL                                     |
|                                                                       |
|   $ du -sh /tmp /var/tmp 2>/dev/null                                  |
|                                                                       |
|   [ ACCEPT ]              [ REJECT ]              [ ALWAYS ALLOW ]    |
|   Ctrl+Shift+A            Ctrl+Shift+D            Ctrl+Shift+W        |
+-----------------------------------------------------------------------+
```

---

## Quickstart & Installation

Switchplate provides multiple simple installation methods.

### Option 1: Debian / Ubuntu Package (.deb)

Recommended for Debian, Ubuntu, Linux Mint, Pop!_OS, and Kali Linux:

```bash
sudo dpkg -i dist/packages/switchplate_1.0.0_amd64.deb
```

This installs the binary to `/usr/bin/switchplate`, registers desktop launchers, and installs application icons.

### Option 2: Single Standalone Binary

Switchplate is compiled as a self-contained executable with all UI assets embedded:

```bash
# Make executable and run anywhere
chmod +x switchplate
./switchplate
```

### Option 3: Automated User Installer

Run the included installation script to set up user binaries, desktop entries, and systemd daemon management:

```bash
./install.sh
```

To cleanly remove Switchplate and all associated services:

```bash
./uninstall.sh
```

---

## Building from Source

### Prerequisites

- Linux (X11 or Wayland)
- Rust toolchain (`cargo`, `rustc`)
- Node.js (v18+) & `npm`
- WebKit2GTK 4.1 development libraries:
  - Debian/Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libayatana-appindicator3-dev librsvg2-dev`
  - Fedora: `sudo dnf install webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel`
  - Arch Linux: `sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3`

### Build Steps

```bash
# 1. Install frontend dependencies and build Vite bundle
npm install
npm run build

# 2. Build Rust release binary with Tauri
cargo build --release --manifest-path src-tauri/Cargo.toml
strip src-tauri/target/release/switchplate
```

The resulting binary will be located at `src-tauri/target/release/switchplate`.

---

## Configuration

Configuration is stored in `~/.config/switchplate/config.toml`. It is created automatically with sensible defaults upon first run.

```toml
[opencode]
server_url = "http://127.0.0.1:49374"
password_env = "OPENCODE_API_PASSWORD"
request_timeout = 30
auto_reconnect = true

[appearance]
position = "top-center"   # "top-center", "top-right", "top-left"
card_width = 480
card_height = 190
collapsed_width = 280
collapsed_height = 38
offset_x = 0
offset_y = 12
theme = "industrial-swiss"
reduced_motion = false

[behavior]
only_show_on_request = true
pause_monitoring = false
sound_enabled = true
autostart = true

[shortcuts]
open_island = "ctrl+shift+o"
accept = "ctrl+shift+a"
deny = "ctrl+shift+d"
always = "ctrl+shift+w"
open_settings = "ctrl+shift+s"
```

---

## Service Management

Switchplate runs as a lightweight user systemd daemon:

```bash
# Check service status
systemctl --user status switchplate.service

# Restart service
systemctl --user restart switchplate.service

# Stop service
systemctl --user stop switchplate.service

# View live daemon logs
journalctl --user -u switchplate.service -f
```

---

## Roadmap

- [x] Precision Industrial Swiss design system
- [x] Zero-latency instant approval actions
- [x] Studio acoustic bell chime synthesis (Web Audio API)
- [x] Response generation completion popups
- [x] Native Debian (.deb) package distribution
- [x] Single standalone binary distribution
- [ ] Multi-agent relay adapter (Claude Code CLI hook)
- [ ] Google Gemini CLI approval bridge
- [ ] Antigravity native RPC sidecar connector
- [ ] Interactive diff viewer modal

---

## License

MIT License. Designed and engineered for high-performance agentic engineering workflows.
