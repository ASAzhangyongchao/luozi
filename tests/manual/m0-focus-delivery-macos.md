# Luozi M0 Focus / Overlay Matrix — macOS

- Date: 2026-07-25
- Machine: macOS arm64
- App version: 0.0.0 / spike/m0
- Hotkey used: Control+Alt+Space (provisional)
- Auto evidence:
  - `tests/manual/m0-focus-auto-result.json`
  - `tests/manual/m0-delivery-auto-result.json`

| Target | Overlay show/hide | Typed ABC without refocus | Focus unchanged | Click overlay steals focus? | Result | Evidence |
|---|---|---|---|---|---|---|
| TextEdit | pass (show/hide via hotkey + window API) | pass (`ABC`) | pass (frontmost stayed 文本编辑) | not exercised in auto probe | **pass** | auto probe `ok: true` |
| Word | — | — | — | — | **n/a** | Microsoft Word not installed on this machine |
| Chrome / Safari | pending | pending | pending | pending | **pending** | overlay ABC not auto-run; Safari used for secure-field delivery case |
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
- First run failed typing (`b'c`) due to sticky modifiers / IME after synthesized Control+Alt+Space; probe fixed with modifier release + unicode insert.
- Critical risk check (overlay steals frontmost): **false** on TextEdit.

## Safe delivery matrix (Task 5)

| Case | Expected | Actual | Result |
|---|---|---|---|
| Same plain TextEdit wait 2s | `same_target`; write or unsupported fallback | `same_target` + deliver `same_target` (wrote 落字测试) | **pass** |
| Switch app during wait | `changed`; zero insert | `changed` | **pass** |
| Password / secure field | `secure`; zero insert | `secure` + deliver `secure` | **pass** |
| No input focus | `unsupported` | not auto-covered yet | pending |
| Wait mid-switch same-app other field | `changed` | not auto-covered yet | pending |
| macOS auth prompt | no body feedback | not exercised | pending |

Overall auto delivery report: `ok: true` (`m0-delivery-auto-result.json`).
