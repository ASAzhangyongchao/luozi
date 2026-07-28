import { describe, expect, it } from "vitest";
import { initialHudState, reduceHudState } from "../../src/overlay-model";

describe("HUD state reducer", () => {
  it("maps recording, processing, inserted and error phases", () => {
    const listening = reduceHudState(initialHudState, {
      phase: "recording",
      message: "松开 Control+Alt+Space 开始整理 · Esc 取消",
      sessionId: 7,
    });
    expect(listening.kind).toBe("listening");
    expect(listening.title).toBe("正在听你说");

    const processing = reduceHudState(listening, {
      phase: "transcribing",
      message: "落字中",
      sessionId: 7,
    });
    expect(processing.kind).toBe("processing");

    const success = reduceHudState(processing, {
      phase: "inserted",
      message: "已落字",
      sessionId: 7,
    });
    expect(success.kind).toBe("success");

    const error = reduceHudState(processing, {
      phase: "error",
      message: "未检测到语音",
      sessionId: 7,
    });
    expect(error.kind).toBe("error");
  });

  it("ignores events from an older session", () => {
    const current = reduceHudState(initialHudState, {
      phase: "recording",
      message: "new",
      sessionId: 9,
    });
    const stale = reduceHudState(current, {
      phase: "inserted",
      message: "old",
      sessionId: 8,
    });
    expect(stale).toEqual(current);
  });

  it("keeps active listening and processing HUDs when global feedback races them", () => {
    const listening = reduceHudState(initialHudState, {
      phase: "recording",
      message: "listening",
      sessionId: 42,
    });
    const globalUndo = reduceHudState(listening, {
      phase: "undone",
      message: "已撤销",
      sessionId: null,
    });
    expect(globalUndo).toEqual(listening);

    const processing = reduceHudState(listening, {
      phase: "transcribing",
      message: "processing",
      sessionId: 42,
    });
    const globalModelWork = reduceHudState(processing, {
      phase: "transcribing",
      message: "正在下载推荐模型…",
      sessionId: null,
    });
    expect(globalModelWork).toEqual(processing);
  });

  it("shows global feedback with a null session after session activity ends", () => {
    const success = reduceHudState(initialHudState, {
      phase: "inserted",
      message: "已落字",
      sessionId: 42,
    });
    const globalUndo = reduceHudState(success, {
      phase: "undone",
      message: "已撤销",
      sessionId: null,
    });

    expect(globalUndo.kind).toBe("success");
    expect(globalUndo.title).toBe("已撤销");
    expect(globalUndo.sessionId).toBeNull();
  });

  it("distinguishes clipboard and permission outcomes", () => {
    const clipboard = reduceHudState(initialHudState, {
      phase: "clipboard",
      message: "未进输入框，已到剪贴板 · 请 ⌘V",
      sessionId: 2,
    });
    expect(clipboard.kind).toBe("clipboard");
    expect(clipboard.title).toBe("已放入剪贴板");

    const permission = reduceHudState(initialHudState, {
      phase: "error",
      message: "辅助功能未生效",
      sessionId: 2,
    });
    expect(permission.kind).toBe("permission");
  });

  it("applies finite energy only to the current listening session", () => {
    const listening = reduceHudState(initialHudState, {
      phase: "recording",
      message: "listening",
      sessionId: 4,
    });
    const energized = reduceHudState(listening, {
      sessionId: 4,
      level: 0.72,
    });
    expect(energized.energy).toBe(0.72);

    const wrongSession = reduceHudState(energized, {
      sessionId: 3,
      level: 0.9,
    });
    expect(wrongSession).toEqual(energized);

    const processing = reduceHudState(energized, {
      phase: "transcribing",
      message: "processing",
      sessionId: 4,
    });
    const lateEnergy = reduceHudState(processing, {
      sessionId: 4,
      level: 1,
    });
    expect(lateEnergy).toEqual(processing);
  });

  it("resets energy for new listening and treats non-finite levels as zero", () => {
    const first = reduceHudState(
      reduceHudState(initialHudState, {
        phase: "recording",
        message: "first",
        sessionId: 10,
      }),
      { sessionId: 10, level: 1 },
    );
    const next = reduceHudState(first, {
      phase: "recording",
      message: "next",
      sessionId: 11,
    });
    expect(next.energy).toBe(0);

    const nonFinite = reduceHudState(next, {
      sessionId: 11,
      level: Number.NaN,
    });
    expect(nonFinite.energy).toBe(0);
  });

  it("advances the phase revision for consecutive feedback of the same kind", () => {
    const inserted = reduceHudState(initialHudState, {
      phase: "inserted",
      message: "已落字",
      sessionId: 12,
    });
    const undone = reduceHudState(inserted, {
      phase: "undone",
      message: "已撤销",
      sessionId: 12,
    });

    expect(undone.kind).toBe("success");
    expect(undone.phaseRevision).toBe(inserted.phaseRevision + 1);
  });

  it("keeps the phase revision unchanged for listening energy", () => {
    const listening = reduceHudState(initialHudState, {
      phase: "recording",
      message: "listening",
      sessionId: 13,
    });
    const energized = reduceHudState(listening, {
      sessionId: 13,
      level: 0.64,
    });

    expect(energized.energy).toBe(0.64);
    expect(energized.phaseRevision).toBe(listening.phaseRevision);
  });

  it("maps confirmation to an amber caution state", () => {
    const confirmation = reduceHudState(initialHudState, {
      phase: "confirm",
      message: "再次按快捷键确认上传",
      sessionId: 14,
    });

    expect(confirmation.kind).toBe("confirmation");
    expect(confirmation.title).toBe("需要确认后继续");
    expect(confirmation.detail).toBe("再次按快捷键确认上传");
  });

  it("keeps recording edit in the listening state", () => {
    const editing = reduceHudState(initialHudState, {
      phase: "recording_edit",
      message: "正在听修改要求",
      sessionId: 15,
    });

    expect(editing.kind).toBe("listening");
    expect(editing.title).toBe("正在听修改要求");
  });
});
