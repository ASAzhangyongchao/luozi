# Luozi M0 Focus / Overlay Matrix — macOS

- Date: 2026-07-25
- Machine: macOS arm64
- App version: 0.0.0 / spike/m0
- Tested implementation commit: `cc1038b`
- Hotkey used: Control+Alt+Space (provisional)
- Auto evidence:
  - `tests/manual/m0-focus-auto-result.json`
  - `tests/manual/m0-delivery-auto-result.json`
  - `tests/manual/m0-safety-extra-result.json`

| Target | Overlay show/hide | Typed ABC without refocus | Focus unchanged | Click overlay steals focus? | Result | Evidence |
|---|---|---|---|---|---|---|
| TextEdit | pass (show/hide via hotkey + window API) | pass once (`ABC`); one setup-timing run returned `BC` | pass in both runs (frontmost stayed 文本编辑) | covered separately with Safari frontmost | **pass (focus invariant)** | latest auto probe `ok: true`; first rerun exposed input-setup timing flake |
| Word | — | — | — | — | **n/a** | Microsoft Word not installed on this machine |
| Chrome / Safari | pass (Safari) | pass (new run appended `ABC` suffix without refocus) | pass | pass (Safari stayed frontmost after clicking overlay center) | **pass (Safari)** | `m0-safety-extra-result.json`; `m0-overlay-opaque-fallback.png` |
| VS Code | — | — | — | — | **n/a** | VS Code not installed (JoyCode/OpenCode/Xcode present only) |

## Procedure

1. Register hotkey in Luozi main window.
2. Focus target app text field and type `A`.
3. Hold hotkey ~1s — overlay should appear; release — overlay hides.
4. Type `B` without clicking the target again.
5. Short-press hotkey once, then type `C`.
6. Expect continuous `ABC` and same focused field.

Pass only if overlay does not steal focus.

## Auto probe notes (Task 4)

- Accessibility trusted for Luozi: yes.
- Focus invariant remained pass on two TextEdit reruns. One run lost the initial `A` before overlay activity; the next run produced `ABC`, so the remaining flake is in fixed-delay test setup rather than observed focus stealing.
- Critical risk check (overlay steals frontmost): **false** on TextEdit and Safari.
- Clicking the visible 360 × 72 overlay at its center kept Safari frontmost.

## Safe delivery matrix (Task 5)

| Case | Expected | Actual | Result |
|---|---|---|---|
| Same plain TextEdit wait 2s | `same_target`; write or unsupported fallback | `same_target` + deliver `same_target` (wrote 落字测试) | **pass** |
| Switch app during wait | `changed`; zero insert | `changed` | **pass** |
| Password / secure field | `secure`; zero insert | `secure` + deliver `secure` | **pass** |
| No input focus | `unsupported` | button remained focused; validation `same_target`; deliver `unsupported`; both text fields empty | **pass** |
| Wait mid-switch same-app other field | `changed` | focus moved from first to second Safari field after capture; validation `changed`; both fields empty | **pass** |
| macOS auth prompt | no body feedback | not exercised | pending |

Core auto delivery report: `ok: true` (`m0-delivery-auto-result.json`). Extra Safari safety cases pass in `m0-safety-extra-result.json`. macOS system authentication remains the only untested safety branch in this matrix.
