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

- Duplicate stop during ASR no longer cancels in-flight Whisper (Released / auto-stop races)
- ASR no longer holds the engine mutex across multi-second `full()`; Esc can mark stale
- Empty mic buffers fail early; multi-segment transcripts join with spaces
- Tray shows model-missing until `ggml-small.bin` is present

### Notes

- No public release artifacts yet
- Shortcuts remain provisional (Mac-first Partial M0)
- Build requires `cmake` for `whisper-rs-sys`
- Model file is not in Git; missing model surfaces `model_missing` instead of fake text
