export type HudKind =
  | "idle"
  | "listening"
  | "processing"
  | "success"
  | "clipboard"
  | "confirmation"
  | "permission"
  | "error";

export type HudPhaseEvent = {
  phase: string;
  message: string;
  sessionId: number | null;
};

export type HudEnergyEvent = {
  sessionId: number;
  level: number;
};

export type HudEvent = HudPhaseEvent | HudEnergyEvent;

export type HudState = {
  kind: HudKind;
  title: string;
  detail: string;
  sessionId: number | null;
  energy: number;
  phaseRevision: number;
};

export const initialHudState: HudState = {
  kind: "idle",
  title: "Luozi 已就绪",
  detail: "",
  sessionId: null,
  energy: 0,
  phaseRevision: 0,
};

export function reduceHudState(current: HudState, event: HudEvent): HudState {
  if ("level" in event) {
    if (current.kind !== "listening" || event.sessionId !== current.sessionId) {
      return current;
    }
    const level = Number.isFinite(event.level) ? event.level : 0;
    return {
      ...current,
      energy: Math.min(1, Math.max(0, level)),
    };
  }

  if (
    current.sessionId !== null &&
    event.sessionId !== null &&
    event.sessionId < current.sessionId
  ) {
    return current;
  }

  const sessionId = event.sessionId ?? current.sessionId;
  const phaseRevision = current.phaseRevision + 1;
  switch (event.phase) {
    case "recording":
    case "recording_edit":
      return {
        kind: "listening",
        title: event.phase === "recording_edit" ? "正在听修改要求" : "正在听你说",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    case "transcribing":
    case "delivering":
      return {
        kind: "processing",
        title: event.message.includes("修改") ? "正在修改表达" : "正在整理表达",
        detail: "已经停止录音，正在准备写入",
        sessionId,
        energy: 0,
        phaseRevision,
      };
    case "inserted":
    case "undone":
      return {
        kind: "success",
        title: event.phase === "undone" ? "已撤销" : "已写入当前输入框",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    case "clipboard":
      return {
        kind: "clipboard",
        title: "已放入剪贴板",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    case "confirm":
      return {
        kind: "confirmation",
        title: "需要确认后继续",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    case "error": {
      const needsPermission =
        event.message.includes("辅助功能") || event.message.includes("permission");
      return {
        kind: needsPermission ? "permission" : "error",
        title: needsPermission ? "需要检查系统权限" : "这段没有处理成功",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    }
    case "rejected":
    case "discarded":
    case "too_short":
    case "warn":
      return {
        kind: "error",
        title: event.phase === "too_short" ? "请再按久一点" : "本次没有写入",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    case "canceled":
      return {
        kind: "idle",
        title: "已取消",
        detail: event.message,
        sessionId,
        energy: 0,
        phaseRevision,
      };
    default:
      return {
        kind: "processing",
        title: event.message || "正在处理",
        detail: "",
        sessionId,
        energy: 0,
        phaseRevision,
      };
  }
}
