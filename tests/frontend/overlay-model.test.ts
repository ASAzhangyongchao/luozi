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

  it("ignores events from an older session and inherits a null session id", () => {
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

    const inherited = reduceHudState(current, {
      phase: "transcribing",
      message: "processing",
      sessionId: null,
    });
    expect(inherited.sessionId).toBe(9);
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
});
