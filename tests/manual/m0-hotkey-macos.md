# Luozi M0 Hotkey Matrix — macOS

- Date: 2026-07-25
- Machine: macOS 26.5.2 arm64
- App version: 0.0.0
- Commit: (see git log at fill time)

| Action | Candidate | Short events | Hold events | IME | Word | Browser | VS Code | Restart | Result | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| Continue speaking | Fn | n/a | n/a | n/a | n/a | n/a | n/a | n/a | **fail** | Registration failed: Couldn't recognize "Fn" |
| Continue speaking | Control+Alt+Space | pass (paired Pressed/Released ×2 shown) | pending full 10× | pending | pending | pending | pending | pending | **pass (events)** | Screenshot: `control+alt+Space` Pressed/Released pairs |
| Continue speaking | Control+Shift+Space | pass (1 pair shown) | pending full 10× | pending | pending | pending | pending | pending | **pass (events)** | Screenshot: `shift+control+Space` Pressed/Released |
| Voice edit | Control+Alt+M | pass (3 pairs shown) | pending full 10× | pending | pending | pending | pending | pending | **pass (events)** | Screenshot: `control+alt+KeyM` Pressed/Released pairs |
| Voice edit | Control+Shift+M | pass (2 pairs shown) | pending full 10× | pending | pending | pending | pending | pending | **pass (events)** | Screenshot: `shift+control+KeyM` Pressed/Released pairs |

## Provisional default (macOS, pending conflict matrix)

Per candidate order in the roadmap (skip failed `Fn`):

| Action | Provisional default |
|---|---|
| Continue speaking | `Control+Alt+Space` |
| Voice edit | `Control+Alt+M` |

## Notes

- Official plugin delivers both `Pressed` and `Released` for the four non-Fn candidates.
- Full Go still needs: short/hold 10× counts, IME / Word / Browser / VS Code conflict check, and restart re-register.
- Do not add low-level hooks for `Fn` in M0.
