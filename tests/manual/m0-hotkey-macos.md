# Luozi M0 Hotkey Matrix — macOS

- Date: 2026-07-25
- Machine: macOS 26.5.2 arm64
- App version: 0.0.0
- Tested implementation commit: `cc1038b` (`codex/m0-mac-closure`)
- Active input source during automated matrix: `com.tencent.inputmethod.wetype.pinyin`

| Action | Candidate | Short 10/10 | Hold 1s 10/10 | Hold 5s 10/10 | IME | Word | Browser | VS Code | Restart | Result | Evidence |
|---|---|---:|---:|---:|---|---|---|---|---|---|---|
| Continue speaking | Fn | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | **fail** | Registration failed: Couldn't recognize "Fn" |
| Continue speaking | Control+Alt+Space | **pass** | **pass** | **pass** | **pass** (WeType pinyin active) | n/a | **pass** (Safari local fixture) | n/a | **pass** | `m0-hotkey-auto-result.json`; exactly 10 Pressed + 10 Released in each duration, no duplicates |
| Continue speaking | Control+Shift+Space | pending full matrix (prior 1 pair) | pending | pending | pending | n/a | pending | n/a | pending | **partial** | Prior event screenshot only |
| Voice edit | Control+Alt+M | pending full physical-key matrix (prior 3 pairs) | pending | pending | pending | n/a | pending | n/a | registration pass; event restart pending | **partial** | Registration confirmed; synthetic letter-key driver was inconclusive |
| Voice edit | Control+Shift+M | pending full matrix (prior 2 pairs) | pending | pending | pending | n/a | pending | n/a | pending | **partial** | Prior event screenshot only |

## Provisional default (macOS, pending conflict matrix)

Per candidate order in the roadmap (skip failed `Fn`):

| Action | Provisional default |
|---|---|
| Continue speaking | `Control+Alt+Space` |
| Voice edit | `Control+Alt+M` |

## Notes

- `Control+Alt+Space` now has complete short / 1s / 5s automated counts and restart re-registration evidence.
- Safari ABC and the full Space matrix ran while WeType pinyin was the selected input source.
- `Control+Alt+M` still needs physical-key short / 1s / 5s counts; synthetic letter-key input did not exercise the registered shortcut reliably.
- Word and VS Code are not installed. JoyCode / OpenCode are present but are not recorded as substitutes for the fixed matrix.
- Do not add low-level hooks for `Fn` in M0.
