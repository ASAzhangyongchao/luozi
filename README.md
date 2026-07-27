# Luozi (落字)

Cross-platform voice-to-text desktop app (Tauri 2 + Rust).

**Status:** Mac-first **M6 draft workbench** (edit / paste / copy / autosave / undo; dictation into the focused draft). M5 Groq cloud ASR still available. There is **no downloadable release** yet.

## What works today

- Hold `Control+Alt+Space`: transcript → draft when the workbench is focused, otherwise AX / clipboard
- Tray: **Open voice draft**, ASR mode, Groq Key / consent, download model
- Manual checklists: `docs/spikes/m5-manual.md`, `docs/spikes/m6-manual.md`

**Cloud:** Keys only in Keychain; no upload without consent. Auto mode does **not** upload when the local model is ready.

## Develop

```bash
brew install cmake
npm ci
npm run fetch:model
npm run tauri dev
cargo test --workspace
npm run install:macos-app
```

## Not yet

Voice edit / text AI (M7), full Settings UI (M8), notarized installers, GitHub Release artifacts.

## License

MIT — see [LICENSE](./LICENSE).
