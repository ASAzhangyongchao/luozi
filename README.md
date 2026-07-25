# Luozi (落字)

Cross-platform voice-to-text desktop app (Tauri 2 + Rust).

**Status:** Mac-first M1 skeleton. There is **no downloadable release** yet.

## What works today

- Local desktop shell with tray menu (Practice / About / Quit)
- Shared `luozi-core` `AppConfig` with `schemaVersion = 1`
- macOS M0 Spike evidence under `docs/spikes/` and `tests/manual/` (Overall **Partial**; Windows not verified on a real machine)

## Shortcuts (provisional)

Until dual-platform M0 Go, treat these as **provisional**, not a product promise:

| Action | Provisional binding (macOS) |
|---|---|
| Continue speaking | `Control+Alt+Space` |
| Voice edit | `Control+Alt+M` |

## Develop

```bash
# Node 20+
npm ci
npm run tauri dev
```

M0 Spike console: open the practice window with `?spike` in the URL (dev tools / custom link), or keep using Spike env autos from M0 docs.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run build
```

## Not in M1

Session state machine, ASR, workbench, text AI, public GitHub push, notarized installers.

## License

MIT — see [LICENSE](./LICENSE).
