# Luozi (落字)

Cross-platform voice-to-text desktop app (Tauri 2 + Rust).

**Status:** Mac-first **M3 local Whisper** (`ggml-small`, manual model). There is **no downloadable release** yet.

## What works today

- Hold `Control+Alt+Space`: real mic → local Whisper transcript → AX insert / clipboard+⌘V / clipboard
- Esc (registered only while a session is active): cancel
- Tray: start / cancel / undo last clipboard delivery / practice / about / quit
- `luozi-core` session state machine tests + `AppConfig` `schemaVersion = 1`
- Falling Cursor brand assets under `assets/brand/`
- Manual checklists: `docs/spikes/m2-manual.md` (session), `docs/spikes/m3-manual.md` (Whisper)
- M0 Spike evidence under `docs/spikes/` and `tests/manual/` (Overall **Partial**)

**Permissions:** Accessibility / Microphone must be granted to the **process you run**. `npm run tauri dev` uses `target/debug/luozi`; the bundled **`Luozi.app`** is a different signature — toggles do not share.

**Model:** place `ggml-small.bin` at  
`~/Library/Application Support/app.luozi.desktop/models/ggml-small.bin`  
(or `npm run fetch:model`). Override with `LUOZI_WHISPER_MODEL`. Build needs `cmake` (`brew install cmake`).

## Brand

Visual system: **Falling Cursor** (夜青 `#0B3034` + 极光青 `#42D9D3` + 冰白 `#F4FBFA`).

## Shortcuts (provisional)

| Action | Provisional binding (macOS) |
|---|---|
| Continue speaking (hold) | `Control+Alt+Space` |
| Cancel | `Esc` (while session active) |
| Voice edit | `Control+Alt+M` (not enabled yet) |

## Develop

```bash
brew install cmake          # once, for whisper-rs
npm ci
npm run fetch:model         # ~466MB ggml-small.bin
npm run tauri dev
cargo test --workspace
npm run run:macos-app       # build + open Luozi.app (stable TCC check)
```

## Not yet

In-app model downloader (M4), cloud ASR, workbench, voice edit, public GitHub push, notarized installers.

## License

MIT — see [LICENSE](./LICENSE).
