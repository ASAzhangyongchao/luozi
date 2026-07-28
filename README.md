# Luozi (落字)

Cross-platform voice-to-text desktop app (Tauri 2 + Rust).

**Status:** Mac-first **M8 settings shell (shortest)** — dedicated Settings / About window for ASR, text AI, hotkeys (read-only), and permissions. M7 voice edit + M6 draft + M5 Groq ASR still available. **No downloadable Release yet.**

## What works today

- Hold `Control+Alt+Space`: transcript → draft when workbench focused, else AX / clipboard
- Hold `Control+Alt+Shift+Space`: speak an edit instruction → text AI rewrites the draft scope
- Tray: **left-click** opens draft; **right-click** menu has Settings / How to use / engine. Dock click also opens draft. Draft left rail also opens Settings and the How-to guide.
- Manual checklists: `docs/spikes/m5-manual.md` … `m8-manual.md`, `guide-manual.md`

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

First-run auto onboarding (in-app tray guide available), liquid-glass polish, hotkey remapping UI, GitHub Release installers, notarization.

## License

MIT — see [LICENSE](./LICENSE).
