# Luozi M0 Focus / Overlay Matrix — macOS

- Date:
- Machine: macOS arm64
- App version: 0.0.0
- Hotkey used: Control+Alt+Space (provisional)

| Target | Overlay show/hide | Typed ABC without refocus | Focus unchanged | Click overlay steals focus? | Result | Evidence |
|---|---|---|---|---|---|---|
| TextEdit | | | | | | |
| Word | | | | | | |
| Chrome / Safari | | | | | | |
| VS Code | | | | | | |

## Procedure

1. Register hotkey in Luozi main window.
2. Focus target app text field and type `A`.
3. Hold hotkey ~1s — overlay should appear; release — overlay hides.
4. Type `B` without clicking the target again.
5. Short-press hotkey once, then type `C`.
6. Expect continuous `ABC` and same focused field.

Pass only if overlay does not steal focus.
