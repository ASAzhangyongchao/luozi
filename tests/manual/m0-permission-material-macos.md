# Luozi M0 Permission / Material Matrix — macOS

- Date: 2026-07-25
- Machine: macOS arm64
- App version: 0.0.0 / spike/m0
- Tested implementation commit: `cc1038b`
- Auto evidence:
  - `tests/manual/m0-audio-auto-result.json`
  - `tests/manual/m0-overlay-cycle-result.json`
  - `tests/manual/m0-resource-auto-result.json`

## Microphone

| Case | Result | Evidence / notes |
|---|---|---|
| First allow / record 1s | **pass** | 48000 Hz, 1 ch, 48128 frames, 192580 bytes |
| Denied in System Settings then retry | pending | requires manual toggle |
| No input device | pending | not simulated on this machine |
| Disconnect during record | pending | not simulated |
| Temp WAV deleted after probe | **pass** | `deleted: true` |
| Leftover WAV in `/tmp/luozi-m0-audio` | **pass** | directory absent after probe |
| `NSMicrophoneUsageDescription` in `.app` | **pass** | zh+en string present in bundled Info.plist |

## Overlay material

| Item | Value |
|---|---|
| Material name | Standard opaque Canvas fallback |
| Private API | **disabled**: no `app.macOSPrivateApi`; no Tauri `macos-private-api` feature |
| Native Liquid Glass / vibrancy | pending narrow public AppKit bridge; not claimed by M0 |
| Reduced transparency fallback | same opaque high-contrast Canvas, so no transparency-dependent failure |
| Steals focus on show/hide or click | no (TextEdit and Safari probes) |
| 30 show/hide cycles | **pass** (`ok: true`, 2592 ms) |
| Idle 5 min CPU | Release Spike main window open: main P95 0.3%; related-process total P95 0.5% |
| Idle memory | `top` related total 74.53 → 74.47 MiB, max 87.52 MiB; no continuous growth |
| RSS snapshots | related total 164.16 MiB at start → 107.78 MiB at end |
| Screenshot of degrade | `m0-overlay-opaque-fallback.png` |
| Limitation | This is not final tray-only idle; per-process GPU was not instrumented |

## Build smoke

| Item | Result |
|---|---|
| `npm run build` | **pass** |
| `cargo check` | **pass** |
| `cargo fmt` | **pass** (applied) |
| `cargo clippy -D warnings` | **pass** |
| `npm run tauri build` | **pass outside the command sandbox**; sandboxed `hdiutil` returned “设备未配置” |
| Artifact path | `src-tauri/target/release/bundle/macos/Luozi.app` ; `src-tauri/target/release/bundle/dmg/Luozi_0.0.0_aarch64.dmg` |
| Launch / restart | **pass**; packaged App launched, quit, relaunched, and auto-registered the default Space shortcut |
| Gatekeeper note | ad-hoc / linker-signed (`Signature=adhoc`); `spctl` accepted only with `override=security disabled` on this machine; no distribution claim |
