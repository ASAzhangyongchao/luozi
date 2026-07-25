# Luozi M0 Permission / Material Matrix — macOS

- Date: 2026-07-25
- Machine: macOS arm64
- App version: 0.0.0 / spike/m0
- Auto evidence:
  - `tests/manual/m0-audio-auto-result.json`
  - `tests/manual/m0-overlay-cycle-result.json`

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
| Material name | CSS semantic blur (`backdrop-filter` + translucent Canvas) |
| Private vibrancy API | not used for material (window still uses existing transparent + `macOSPrivateApi` from Task 4) |
| Reduced transparency fallback | `@media (prefers-reduced-transparency: reduce)` → opaque Canvas |
| Steals focus on show/hide | no (Task 4 TextEdit probe) |
| 30 show/hide cycles | **pass** (`ok: true`, 2590 ms) |
| Idle 60s CPU | pending (not instrumented) |
| Screenshot of degrade | pending |

## Build smoke

| Item | Result |
|---|---|
| `npm run build` | **pass** |
| `cargo check` | **pass** |
| `cargo fmt` | **pass** (applied) |
| `cargo clippy -D warnings` | **pass** |
| `npm run tauri build` | **pass** |
| Artifact path | `src-tauri/target/release/bundle/macos/Luozi.app` ; `src-tauri/target/release/bundle/dmg/Luozi_0.0.0_aarch64.dmg` |
| Gatekeeper note | ad-hoc / linker-signed (`Signature=adhoc`); `spctl` reported accepted with `override=security disabled` on this machine |
