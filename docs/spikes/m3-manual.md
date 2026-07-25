# Luozi M3 Manual Check (Mac-first local Whisper)

**Scope:** real mic → local `whisper.cpp` (`ggml-small`) → insert/clipboard. No cloud ASR. No in-app downloader (M4).

## Prep

1. Install build tool once: `brew install cmake` (required to compile `whisper-rs`).
2. Place model (or run helper):

```bash
npm run fetch:model
# → ~/Library/Application Support/app.luozi.desktop/models/ggml-small.bin
```

3. Grant Microphone + Accessibility to the process you run (`tauri dev` → `debug/luozi`, or `Luozi.app`).
4. `npm run tauri dev` from `apps/luozi`.
5. Open TextEdit, focus a plain text field.

## Cases

| # | Steps | Expect |
|---|---|---|
| 1 | Hold `Control+Alt+Space` ≥0.5s, speak a short Chinese phrase, release | Overlay「落字中」→ caret gets real transcript (not `落字测试`) |
| 2 | Remove/rename model file, try again | Overlay error `model_missing: …`; no fake text |
| 3 | Hold hotkey, `Esc` before release | Cancel; no deliver |
| 4 | Tap <0.3s | 「时间太短」 |

## Notes

- Default model path override: env `LUOZI_WHISPER_MODEL=/path/to/ggml-small.bin`
- Language follows `AppConfig.language` (`auto` by default).
- Delivery still AX → clipboard+⌘V → clipboard only.
