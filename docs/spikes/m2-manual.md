# Luozi M2 Manual Check (Mac-first)

**Scope:** real mic session + fake transcript `落字测试` + insert/clipboard/undo. No ASR.

## Prep — pick one identity

macOS Accessibility / Microphone toggles bind to the **running code signature**, not the product name. These two are **different**:

| Mode | How to run | Grant permissions to |
|---|---|---|
| Hot iteration | `npm run tauri dev` | bare binary `target/debug/luozi` (Finder → add this file) |
| Stable TCC check | `npm run run:macos-app` | bundled **`Luozi.app`** |

If Accessibility shows `Luozi.app` ON but you are on `tauri dev`, insert will fail until you also enable the debug `luozi` binary (or switch to the bundled app).

1. Grant **Microphone** + **Accessibility** to the identity you will actually run.
2. Open TextEdit, click into a plain text document.
3. Run either `npm run tauri dev` or `npm run run:macos-app` from `apps/luozi`.

## Delivery path

On release (≥0.3s hold): prefer AX insert into the focused field → else clipboard + synthesized ⌘V → else clipboard only (manual ⌘V).

## Cases

| # | Steps | Expect |
|---|---|---|
| 1 | Hold `Control+Alt+Space` ≥0.3s, release（练习窗会显示实际已注册键） | Overlay「听写中」→ caret shows `落字测试`（may arrive via ⌘V fallback） |
| 2 | Hold hotkey, press `Esc` before release | No insert; overlay closes;「已取消」 |
| 3 | Start in TextEdit, switch to Safari before release | Clipboard gets `落字测试`; tray「撤销上次落字」restores previous clip within 60s |
| 4 | Focus a password field, press hotkey | Reject start; no clip write |
| 5 | Tray → 开始语音输入, then 取消当前录音 | Same as cancel |
| 6 | Tap hotkey <0.3s | 「时间太短」; no deliver |

## Notes

- Shortcuts still **provisional** (M0 Partial).
- Escape is registered only while a session is active.
- Engine menu shows「假文本（M2）」until M3 Whisper.
- Existing bundle path (if already built): `src-tauri/target/release/bundle/macos/Luozi.app` — you can `open` it without rebuilding.
