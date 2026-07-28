import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles-settings.css";

type SettingsSnapshot = {
  version: string;
  asrModeLabel: string;
  asrMode: string;
  modelStatus: string;
  modelReady: boolean;
  cloudAsrReady: boolean;
  cloudAsrHost: string;
  cloudAsrProvider: string;
  textAiReady: boolean;
  textAiHost: string;
  textAiProvider: string;
  textAiModel: string;
  continueSpeakingShortcut: string;
  voiceEditShortcut: string;
  registeredContinue: string | null;
  accessibilityTrusted: boolean;
  holdToTalk: boolean;
  repoUrl: string;
  releasesUrl: string;
};

type InitOptions = {
  root?: ParentNode;
  embedded?: boolean;
  initialSection?: string;
  bindWindowClose?: boolean;
};

const boundRoots = new WeakSet<ParentNode>();

function toastEl(root: ParentNode) {
  return root.querySelector<HTMLParagraphElement>("#settingsToast");
}

function showToast(root: ParentNode, msg: string) {
  const el = toastEl(root);
  if (!el) return;
  el.textContent = msg;
  setTimeout(() => {
    if (el.textContent === msg) el.textContent = "";
  }, 2200);
}

function showSection(root: ParentNode, id: string) {
  root.querySelectorAll<HTMLElement>(".settings-section").forEach((el) => {
    el.hidden = el.id !== `section-${id}`;
  });
  root.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.section === id);
  });
}

function applySnapshot(root: ParentNode, s: SettingsSnapshot) {
  const asrHint = root.querySelector("#asrModeHint");
  if (asrHint) asrHint.textContent = s.asrModeLabel;
  const modelStatus = root.querySelector("#modelStatus");
  if (modelStatus) modelStatus.textContent = s.modelStatus;
  const modelBadge = root.querySelector("#modelBadge");
  if (modelBadge) {
    modelBadge.textContent = s.modelReady ? "就绪" : "未就绪";
    modelBadge.className = `badge ${s.modelReady ? "ok" : ""}`;
  }
  const cloud = root.querySelector("#cloudAsrStatus");
  if (cloud) {
    cloud.textContent = s.cloudAsrReady
      ? `已就绪 · ${s.cloudAsrHost || s.cloudAsrProvider}`
      : `未就绪 · 需 Key + 同意（${s.cloudAsrHost || "api.groq.com"}）`;
  }
  const textAi = root.querySelector("#textAiStatus");
  if (textAi) {
    textAi.textContent = s.textAiReady
      ? `已就绪 · ${s.textAiModel} @ ${s.textAiHost || s.textAiProvider}`
      : `未就绪 · 需 Key + 同意（${s.textAiHost || "api.groq.com"}）`;
  }
  const hkC = root.querySelector("#hkContinue");
  if (hkC) hkC.textContent = s.continueSpeakingShortcut;
  const hkE = root.querySelector("#hkEdit");
  if (hkE) hkE.textContent = s.voiceEditShortcut;
  const hkR = root.querySelector("#hkRegistered");
  if (hkR) {
    hkR.textContent = s.registeredContinue
      ? `实际注册的「继续说」：${s.registeredContinue}`
      : "继续说热键尚未注册成功时，可用托盘「开始语音输入」";
  }
  const ax = root.querySelector("#axStatus");
  if (ax) {
    ax.textContent = s.accessibilityTrusted
      ? "辅助功能：已生效（可尝试插入焦点）"
      : "辅助功能：未生效 — 请打开系统设置并开关一次「落字」";
  }
  const ver = root.querySelector("#aboutVersion");
  if (ver) ver.textContent = s.version;
}

async function refresh(root: ParentNode) {
  const snap = await invoke<SettingsSnapshot>("settings_snapshot");
  applySnapshot(root, snap);
  return snap;
}

function bindHandlers(root: ParentNode) {
  root.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.addEventListener("click", () => showSection(root, btn.dataset.section || "general"));
  });

  root.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const id = btn.dataset.copy!;
      const code = root.querySelector(`#${id}`)?.textContent || "";
      try {
        await navigator.clipboard.writeText(code);
        showToast(root, "已复制");
      } catch {
        showToast(root, "复制失败");
      }
    });
  });

  root.querySelector("#btnCycleAsr")?.addEventListener("click", async () => {
    try {
      await invoke("settings_cycle_asr_mode");
      await refresh(root);
      showToast(root, "已切换引擎模式");
    } catch (e) {
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnAsrKey")?.addEventListener("click", async () => {
    try {
      await invoke("settings_prompt_asr_key");
      await refresh(root);
      showToast(root, "ASR Key 已保存");
    } catch (e) {
      if (String(e).includes("canceled")) return;
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnAsrConsent")?.addEventListener("click", async () => {
    try {
      await invoke("settings_consent_asr");
      await refresh(root);
      showToast(root, "已同意 ASR 上传");
    } catch (e) {
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnTextAiKey")?.addEventListener("click", async () => {
    try {
      await invoke("settings_prompt_text_ai_key");
      await refresh(root);
      showToast(root, "文本 AI Key 已保存");
    } catch (e) {
      if (String(e).includes("canceled")) return;
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnTextAiConsent")?.addEventListener("click", async () => {
    try {
      await invoke("settings_consent_text_ai");
      await refresh(root);
      showToast(root, "已同意文本 AI 上传");
    } catch (e) {
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnOpenMic")?.addEventListener("click", async () => {
    try {
      await invoke("settings_open_microphone");
    } catch (e) {
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnOpenAx")?.addEventListener("click", async () => {
    try {
      await invoke("settings_open_accessibility");
    } catch (e) {
      showToast(root, String(e));
    }
  });
  root.querySelector("#btnRepo")?.addEventListener("click", async () => {
    await invoke("settings_open_repo");
  });
  root.querySelector("#btnReleases")?.addEventListener("click", async () => {
    await invoke("settings_open_releases");
  });
  root.querySelector("#btnSpike")?.addEventListener("click", async () => {
    await invoke("settings_open_spike");
  });
}

export async function initSettingsPage(opts: InitOptions = {}): Promise<void> {
  const root = opts.root ?? document;
  const embedded = Boolean(opts.embedded);

  if (!boundRoots.has(root)) {
    bindHandlers(root);
    boundRoots.add(root);
    if (!embedded) {
      await listen<{ section?: string }>("settings://nav", (ev) => {
        const section = ev.payload?.section || "about";
        showSection(root, section);
      });
    }
  }

  let initial = opts.initialSection || "general";
  if (!embedded) {
    const params = new URLSearchParams(location.search);
    initial = params.get("section") || initial;
    try {
      const pending = await invoke<string | null>("settings_take_nav");
      if (pending) initial = pending;
    } catch {
      /* ignore */
    }
  }

  showSection(root, initial);
  await refresh(root);

  if (opts.bindWindowClose !== false && !embedded) {
    const win = getCurrentWindow();
    await win.onCloseRequested(async (event) => {
      event.preventDefault();
      await win.hide();
    });
  }
}

export function navigateSettings(root: ParentNode, section: string) {
  showSection(root, section);
  void refresh(root);
}

const isSettingsEntry =
  typeof document !== "undefined" && document.body?.classList.contains("settings-body");

if (isSettingsEntry) {
  initSettingsPage().catch((err) => {
    console.error(err);
    document.body.textContent = `设置页加载失败：${err}`;
  });
}
