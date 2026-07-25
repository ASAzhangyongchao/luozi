# Luozi (落字)

Cross-platform voice-to-text desktop app (Tauri 2 + Rust).

**Status:** Mac-first **M2 session skeleton** (fake transcript). There is **no downloadable release** yet.

## What works today

- Hold `Control+Alt+Space`: real mic capture → on release, deliver fake text `落字测试` (AX insert → clipboard + ⌘V → clipboard only)
- Esc (registered only while a session is active): cancel
- Tray: start / cancel / undo last clipboard delivery / practice / about / quit
- `luozi-core` session state machine tests + `AppConfig` `schemaVersion = 1`
- Falling Cursor brand assets under `assets/brand/`
- Manual checklist: `docs/spikes/m2-manual.md`
- M0 Spike evidence under `docs/spikes/` and `tests/manual/` (Overall **Partial**)

**Permissions:** Accessibility / Microphone must be granted to the **process you run**. `npm run tauri dev` uses `target/debug/luozi`; the bundled **`Luozi.app`** is a different signature — toggles do not share.

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
npm ci
npm run tauri dev          # hot iteration (grant AX to debug/luozi)
cargo test --workspace
npm run run:macos-app      # build + open Luozi.app (stable TCC check)
```

If `Luozi.app` already exists under `src-tauri/target/release/bundle/macos/`, you can `open` it directly without rebuilding.

## Not yet

ASR / Whisper, workbench, voice edit, public GitHub push, notarized installers.

## License

MIT — see [LICENSE](./LICENSE).
