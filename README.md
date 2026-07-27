# Luozi (落字)

Cross-platform voice-to-text desktop app (Tauri 2 + Rust).

**Status:** Mac-first **M7 voice edit (shortest path)** — second hotkey + text AI rewrite of selection/paragraph with protected-token confirm. M6 draft + M5 Groq ASR still available. **No downloadable Release yet.**

## What works today

- Hold `Control+Alt+Space`: transcript → draft when workbench focused, else AX / clipboard
- Hold `Control+Alt+Shift+Space`: speak an edit instruction → text AI rewrites the draft scope
- Tray: open draft, ASR mode, ASR/text-AI keys & consent, download model
- Manual checklists: `docs/spikes/m5-manual.md`, `m6-manual.md`, `m7-manual.md`

**Cloud:** ASR and text AI require separate consent; keys only in Keychain.

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

Full suspicion marks / diff sheet, Settings UI (M8), GitHub Release installers, notarization.

## License

MIT — see [LICENSE](./LICENSE).
