# Changelog

All notable changes to Switchplate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.4] - 2026-11-24

### Added
- **Interactive OpenCode Questions:** Support for OpenCode v2 interactive questions and form cards in the floating island.
  - Render single-choice, multiple-choice, boolean toggles, numeric, and text inputs with validation.
  - Dynamic conditional field visibility (`when` clauses evaluated in real time).
  - Inline "Send answers" or "Hide" actions (retains question pending for island or terminal resolution).
  - Fallback notification directing user to terminal if unsupported input schemas are encountered.
- **Auto-Approval Modes:** Configurable behavior under `Settings → Behaviour → OpenCode permission requests`:
  - `manual` ("Ask every time", default): standard manual review for each request.
  - `allow_once` ("Auto-approve once"): automatically dispatches approvals for all pending and incoming requests.
  - `always_allow` ("Auto-approve always"): approves requests and instructs OpenCode to persist rules.
  - Graceful fallback: pauses with monitoring; unhandled automatic dispatches revert to manual review.
- **Native Desktop Audio Engine:**
  - Migrated from Web Audio to native PCM audio synthesis in Rust (`src-tauri/src/sound.rs`).
  - Direct output via Linux desktop audio backends (`pw-play` for PipeWire, `paplay` for PulseAudio, and `aplay` for ALSA).
  - Allows acoustic cues to play reliably even when the island window is hidden, collapsed, or in the background.
  - Synthesized acoustic harmonic chimes for: Request, Question, Completion, Approve, and Reject.
- **Standalone Archive Support:** Enhanced `install.sh` to install directly from unpacked tarball release directories.

### Fixed
- **SSE Stream TCP Chunk Boundary Parsing (Fixes #1):** Replaced line splitting with a stateful byte-buffering stream unfold in `subscribe_events`. Buffers raw bytes across TCP chunk boundaries before decoding complete JSON lines, preventing dropped events and preserving multibyte UTF-8 sequences.
- **Config Corruption Protection & Atomic Writes (Fixes #2):**
  - Atomic configuration saving via PID-tagged temporary files and renames, preventing truncated or corrupted configuration files.
  - Safe error recovery on load: malformed `config.toml` files are backed up to `config.toml.bak` while loading safe in-memory defaults without overwriting user data.

---

## [1.0.1] - 2026-11-01

### Added
- Renamed project to Switchplate with polished hardware-inspired Industrial Swiss aesthetic.
- Response ready popups indicating code generation and execution completion.
- Web Audio harmonic acoustic chimes for notifications.
- Debian `.deb` distribution package and automated install scripts.
- Clean uninstallation script (`uninstall.sh`).
- Single-instance application enforcement via `tauri-plugin-single-instance`.
- MIT License.

### Fixed
- Resolved duplicate titlebars and extraneous close buttons.
- Fixed tray open and quit application lifecycle management.
- Corrected window capability permissions and tray connection status reporting.
- Sanitized single-instance callbacks and duplicate autostart registry entries.

---

## [1.0.0] - 2026-11-01

### Added
- Initial release of OpenCode Notch.
- Floating dynamic top island interface with auto-hide behavior.
- High-contrast matte charcoal and brass design theme.
- Global keyboard shortcuts (`Ctrl+Shift+A` accept, `Ctrl+Shift+D` deny, `Ctrl+Shift+W` always allow).
- OpenCode event monitoring over Server-Sent Events (SSE).
