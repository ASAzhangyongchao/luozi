# Changelog

## Unreleased

### Added

- M2 Mac-first session path: state machine, continuous mic capture, Esc cancel, fake transcript delivery, clipboard fallback + undo
- M3 local Whisper (`whisper-rs` + Metal): `ggml-small` manual model path, resample to 16 kHz, replace fake transcript
- Falling Cursor brand assets, Dock RGBA icon, menu-bar template tray icon
- Cargo workspace with `luozi-core` (`AppConfig` schemaVersion = 1)
- Tray shell aligned to design §6.3 (M2 enables start / cancel / undo)
- `npm run run:macos-app` helper to open bundled `Luozi.app` for stable TCC checks
- `npm run fetch:model` helper to download `ggml-small.bin` into Application Support
- macOS / Windows CI workflow definition
- M0 Spike evidence retained under `docs/spikes/` and `tests/manual/`

### Fixed

- Continue-speaking hotkey no longer deadlocks the AppKit main thread (beachball on press)
- Accessibility capture falls back to NSWorkspace frontmost app when `AXFocusedApplication` has no value
- Clipboard delivery synthesizes ⌘V so transcript can land at the caret without AX set-value
- Docs clarify that `Luozi.app` and `target/debug/luozi` are different TCC identities

### Notes

- No public release artifacts yet
- Shortcuts remain provisional (Mac-first Partial M0)
- Build requires `cmake` for `whisper-rs-sys`
- Model file is not in Git; missing model surfaces `model_missing` instead of fake text
