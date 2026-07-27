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

const toast = () => document.querySelector<HTMLParagraphElement>("#settingsToast")!;

function showToast(msg: string) {
  toast().textContent = msg;
  setTimeout(() => {
    if (toast().textContent === msg) toast().textContent = "";
  }, 2200);
}

function showSection(id: string) {
  document.querySelectorAll<HTMLElement>(".settings-section").forEach((el) => {
    el.hidden = el.id !== `section-${id}`;
  });
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.section === id);
  });
}

function applySnapshot(s: SettingsSnapshot) {
  const asrHint = document.querySelector("#asrModeHint");
  if (asrHint) asrHint.textContent = s.asrModeLabel;
  const modelStatus = document.querySelector("#modelStatus");
  if (modelStatus) modelStatus.textContent = s.modelStatus;
  const modelBadge = document.querySelector("#modelBadge");
  if (modelBadge) {
    modelBadge.textContent = s.modelReady ? "就绪" : "未就绪";
    modelBadge.className = `badge ${s.modelReady ? "ok" : ""}`;
  }
  const cloud = document.querySelector("#cloudAsrStatus");
  if (cloud) {
    cloud.textContent = s.cloudAsrReady
      ? `已就绪 · ${s.cloudAsrHost || s.cloudAsrProvider}`
      : `未就绪 · 需 Key + 同意（${s.cloudAsrHost || "api.groq.com"}）`;
  }
  const textAi = document.querySelector("#textAiStatus");
  if (textAi) {
    textAi.textContent = s.textAiReady
      ? `已就绪 · ${s.textAiModel} @ ${s.textAiHost || s.textAiProvider}`
      : `未就绪 · 需 Key + 同意（${s.textAiHost || "api.groq.com"}）`;
  }
  const hkC = document.querySelector("#hkContinue");
  if (hkC) hkC.textContent = s.continueSpeakingShortcut;
  const hkE = document.querySelector("#hkEdit");
  if (hkE) hkE.textContent = s.voiceEditShortcut;
  const hkR = document.querySelector("#hkRegistered");
  if (hkR) {
    hkR.textContent = s.registeredContinue
      ? `实际注册的「继续说」：${s.registeredContinue}`
      : "继续说热键尚未注册成功时，可用托盘「开始语音输入」";
  }
  const ax = document.querySelector("#axStatus");
  if (ax) {
    ax.textContent = s.accessibilityTrusted
      ? "辅助功能：已生效（可尝试插入焦点）"
      : "辅助功能：未生效 — 请打开系统设置并开关一次「落字」";
  }
  const ver = document.querySelector("#aboutVersion");
  if (ver) ver.textContent = s.version;
}

async function refresh() {
  const snap = await invoke<SettingsSnapshot>("settings_snapshot");
  applySnapshot(snap);
  return snap;
}

async function main() {
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.addEventListener("click", () => showSection(btn.dataset.section || "general"));
  });

  document.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const id = btn.dataset.copy!;
      const code = document.querySelector(`#${id}`)?.textContent || "";
      try {
        await navigator.clipboard.writeText(code);
        showToast("已复制");
      } catch {
        showToast("复制失败");
      }
    });
  });

  document.querySelector("#btnCycleAsr")?.addEventListener("click", async () => {
    try {
      await invoke("settings_cycle_asr_mode");
      await refresh();
      showToast("已切换引擎模式");
    } catch (e) {
      showToast(String(e));
    }
  });
  document.querySelector("#btnAsrKey")?.addEventListener("click", async () => {
    try {
      await invoke("settings_prompt_asr_key");
      await refresh();
      showToast("ASR Key 已保存");
    } catch (e) {
      if (String(e).includes("canceled")) return;
      showToast(String(e));
    }
  });
  document.querySelector("#btnAsrConsent")?.addEventListener("click", async () => {
    try {
      await invoke("settings_consent_asr");
      await refresh();
      showToast("已同意 ASR 上传");
    } catch (e) {
      showToast(String(e));
    }
  });
  document.querySelector("#btnTextAiKey")?.addEventListener("click", async () => {
    try {
      await invoke("settings_prompt_text_ai_key");
      await refresh();
      showToast("文本 AI Key 已保存");
    } catch (e) {
      if (String(e).includes("canceled")) return;
      showToast(String(e));
    }
  });
  document.querySelector("#btnTextAiConsent")?.addEventListener("click", async () => {
    try {
      await invoke("settings_consent_text_ai");
      await refresh();
      showToast("已同意文本 AI 上传");
    } catch (e) {
      showToast(String(e));
    }
  });
  document.querySelector("#btnOpenMic")?.addEventListener("click", async () => {
    try {
      await invoke("settings_open_microphone");
    } catch (e) {
      showToast(String(e));
    }
  });
  document.querySelector("#btnOpenAx")?.addEventListener("click", async () => {
    try {
      await invoke("settings_open_accessibility");
    } catch (e) {
      showToast(String(e));
    }
  });
  document.querySelector("#btnRepo")?.addEventListener("click", async () => {
    await invoke("settings_open_repo");
  });
  document.querySelector("#btnReleases")?.addEventListener("click", async () => {
    await invoke("settings_open_releases");
  });
  document.querySelector("#btnSpike")?.addEventListener("click", async () => {
    await invoke("settings_open_spike");
  });

  await listen<{ section?: string }>("settings://nav", (ev) => {
    const section = ev.payload?.section || "about";
    showSection(section);
  });

  const params = new URLSearchParams(location.search);
  const initial = params.get("section") || "general";
  showSection(initial);

  await refresh();

  const win = getCurrentWindow();
  await win.onCloseRequested(async (event) => {
    event.preventDefault();
    await win.hide();
  });
}

main().catch((err) => {
  console.error(err);
  document.body.textContent = `设置页加载失败：${err}`;
});
