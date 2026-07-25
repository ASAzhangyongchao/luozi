# Changelog

## Unreleased

### Added

- M2 Mac-first session path: state machine, continuous mic capture, Esc cancel, fake transcript delivery, clipboard fallback + undo
- Falling Cursor brand assets, Dock RGBA icon, menu-bar template tray icon
- Cargo workspace with `luozi-core` (`AppConfig` schemaVersion = 1)
- Tray shell aligned to design §6.3 (M2 enables start / cancel / undo)
- `npm run run:macos-app` helper to open bundled `Luozi.app` for stable TCC checks
- macOS / Windows CI workflow definition
- M0 Spike evidence retained under `docs/spikes/` and `tests/manual/`

### Fixed

- Continue-speaking hotkey no longer deadlocks the AppKit main thread (beachball on press)
- Accessibility capture falls back to NSWorkspace frontmost app when `AXFocusedApplication` has no value
- Clipboard delivery synthesizes ⌘V so fake transcript can land at the caret without AX set-value
- Docs clarify that `Luozi.app` and `target/debug/luozi` are different TCC identities

### Notes

- No public release artifacts yet
- Shortcuts remain provisional (Mac-first Partial M0)
- Transcript is still the fixed string `落字测试` until M3 ASR
