# Changelog

## Unreleased

### Added

- Tray left-click and Dock reopen open the voice draft workbench (right-click menu unchanged for Settings / How to use)
- Draft workbench left rail (Typeless-inspired): 草稿 / 历史占位 / 设置 / 教程 — settings & guide open without relying on the menu-bar icon
- In-app beginner guide window from tray 「如何使用…」 (chaptered: start / hotkeys / draft / voice edit / settings / FAQ)
- M8 settings shell (Mac shortest): dedicated Settings window (general / voice+AI / hotkeys read-only / permissions / about); tray Settings & About enabled; Spike demoted to developer entry
- M7 voice edit (Mac shortest): second hotkey `Control+Alt+Shift+Space`, separate text-AI Keychain + consent, OpenAI-compatible chat rewrite, selection/paragraph scope, protected-token high-risk confirm
- M6 draft workbench: autosave, 20-step undo/redo, copy/clear, last-transcript load, dictation into focused draft
- M5 cloud ASR (Mac-first Groq): Keychain, per-provider consent, OpenAI-compatible `/audio/transcriptions`, local-first Auto routing
- M4 model acquisition: embedded `models-manifest.json`, SHA256 verify, tray「下载推荐模型…」, `npm run fetch:model` checksum, idle Whisper unload (≥5m)
- M2 Mac-first session path: state machine, continuous mic capture, Esc cancel, fake transcript delivery, clipboard fallback + undo
- M3 local Whisper (`whisper-rs` + Metal): `ggml-small` model path, resample to 16 kHz, replace fake transcript
- Falling Cursor brand assets, Dock RGBA icon, menu-bar template tray icon
- Cargo workspace with `luozi-core` (`AppConfig` schemaVersion = 1)
- Tray shell aligned to design §6.3 (M2 enables start / cancel / undo)
- `npm run run:macos-app` / `install:macos-app` helpers for stable TCC checks
- macOS / Windows CI workflow definition
- M0 Spike evidence retained under `docs/spikes/` and `tests/manual/`

### Fixed

- Hotkey path hardened: mic starts with zero AX wait; main-thread AX has 250ms timeout; overlay show/hide is non-blocking; stuck-recording watchdog force-cancels
- Duplicate stop during ASR no longer cancels in-flight Whisper (Released / auto-stop races)
- ASR no longer holds the engine mutex across multi-second `full()`; Esc can mark stale
- Empty mic buffers fail early; multi-segment transcripts join with spaces
- Tray shows model-missing until `ggml-small.bin` is present
- AX permission prompts removed from hot path (they beachballed the app)
- Escape stays registered for the whole process life

### Notes

- No public release artifacts / installers yet
- Shortcuts remain provisional (Mac-first Partial M0)
- Build requires `cmake` for `whisper-rs-sys`
- Model file is not in Git; cloud Key never enters config.json / git / logs
- Full Settings UI is M8; M5/M7 use tray + Keychain + consent file
- Text AI consent is separate from ASR consent even on the same host