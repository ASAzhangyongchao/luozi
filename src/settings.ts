import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles-settings.css";

type ProviderOption = {
  id: string;
  label: string;
  help: string;
  supportsCloud: boolean;
};

type SettingsSnapshot = {
  version: string;
  asrModeLabel: string;
  asrMode: string;
  modelStatus: string;
  modelReady: boolean;
  cloudAsrReady: boolean;
  cloudAsrHost: string;
  cloudAsrProvider: string;
  cloudAsrProviderLabel: string;
  cloudAsrModel: string;
  cloudAsrSupports: boolean;
  cloudAsrProviders: ProviderOption[];
  textAiReady: boolean;
  textAiHost: string;
  textAiProvider: string;
  textAiProviderLabel: string;
  textAiModel: string;
  textAiProviders: ProviderOption[];
  continueSpeakingShortcut: string;
  voiceEditShortcut: string;
  registeredContinue: string | null;
  accessibilityTrusted: boolean;
  microphoneAuthorized: boolean;
  microphoneStatus: string;
  holdToTalk: boolean;
  repoUrl: string;
  releasesUrl: string;
};

const toast = () => document.querySelector<HTMLParagraphElement>("#settingsToast")!;
const helpPopover = () => document.querySelector<HTMLDivElement>("#helpPopover")!;
const pickDialog = () => document.querySelector<HTMLDialogElement>("#pickDialog")!;

let permissionsPoll: ReturnType<typeof setInterval> | undefined;
let latest: SettingsSnapshot | null = null;

function showToast(msg: string) {
  toast().textContent = msg;
  setTimeout(() => {
    if (toast().textContent === msg) toast().textContent = "";
  }, 2200);
}

function setQuietButton(btn: HTMLButtonElement | null, quiet: boolean, label: string) {
  if (!btn) return;
  btn.textContent = label;
  btn.classList.toggle("btn-quiet", quiet);
}

function setBadge(el: Element | null, ok: boolean, okText: string, badText: string) {
  if (!el) return;
  el.textContent = ok ? okText : badText;
  el.className = `badge ${ok ? "ok" : "warn"}`;
}

function hideHelp() {
  const pop = helpPopover();
  pop.hidden = true;
  pop.textContent = "";
}

function showHelp(anchor: HTMLElement, text: string) {
  const pop = helpPopover();
  pop.textContent = text;
  pop.hidden = false;
  const rect = anchor.getBoundingClientRect();
  const maxW = 320;
  let left = rect.left;
  if (left + maxW > window.innerWidth - 12) left = window.innerWidth - maxW - 12;
  pop.style.left = `${Math.max(12, left)}px`;
  pop.style.top = `${rect.bottom + 8}px`;
}

function showSection(id: string) {
  hideHelp();
  document.querySelectorAll<HTMLElement>(".settings-section").forEach((el) => {
    el.hidden = el.id !== `section-${id}`;
  });
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    const isActive = btn.dataset.section === id;
    btn.classList.toggle("active", isActive);
    if (isActive) {
      btn.setAttribute("aria-current", "page");
    } else {
      btn.removeAttribute("aria-current");
    }
  });
  const isPrivacy = id === "privacy";
  syncPermissionsPoll(isPrivacy);
  if (isPrivacy) void refresh();
}

function syncPermissionsPoll(active: boolean) {
  if (permissionsPoll !== undefined) {
    clearInterval(permissionsPoll);
    permissionsPoll = undefined;
  }
  if (!active) return;
  permissionsPoll = setInterval(() => {
    void refresh();
  }, 1600);
}

function openPicker(opts: {
  title: string;
  hint: string;
  options: { id: string; label: string; detail?: string; disabled?: boolean }[];
  selectedId?: string;
  onPick: (id: string) => Promise<void>;
}) {
  const dialog = pickDialog();
  const title = document.querySelector("#pickTitle")!;
  const hint = document.querySelector("#pickHint")!;
  const box = document.querySelector("#pickOptions")!;
  title.textContent = opts.title;
  hint.textContent = opts.hint;
  box.innerHTML = "";
  for (const opt of opts.options) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `pick-option${opt.id === opts.selectedId ? " selected" : ""}`;
    btn.disabled = !!opt.disabled;
    btn.innerHTML = `<strong>${opt.label}</strong>${
      opt.detail ? `<span class="muted">${opt.detail}</span>` : ""
    }`;
    btn.addEventListener("click", async () => {
      dialog.close();
      try {
        await opts.onPick(opt.id);
        await refresh();
      } catch (e) {
        if (String(e).includes("canceled")) return;
        showToast(String(e));
      }
    });
    box.appendChild(btn);
  }
  dialog.showModal();
}

function applySnapshot(s: SettingsSnapshot) {
  latest = s;
  const asrHint = document.querySelector("#asrModeHint");
  if (asrHint) asrHint.textContent = s.asrModeLabel;
  const modelStatus = document.querySelector("#modelStatus");
  if (modelStatus) modelStatus.textContent = s.modelStatus;
  const modelBadge = document.querySelector("#modelBadge");
  if (modelBadge) {
    if (s.modelReady) {
      modelBadge.textContent = "已就绪";
      modelBadge.className = "badge ok";
    } else if (s.modelStatus.includes("需重新下载")) {
      modelBadge.textContent = "需重下";
      modelBadge.className = "badge warn";
    } else {
      modelBadge.textContent = "未安装";
      modelBadge.className = "badge";
    }
  }

  const cloud = document.querySelector("#cloudAsrStatus");
  if (cloud) {
    if (!s.cloudAsrSupports) {
      cloud.textContent = `${s.cloudAsrProviderLabel} · 暂无云端 ASR（请用本地或其他厂商）`;
    } else if (s.cloudAsrReady) {
      cloud.textContent = `已就绪 · ${s.cloudAsrProviderLabel} · ${s.cloudAsrModel || s.cloudAsrHost}`;
    } else {
      cloud.textContent = `未就绪 · ${s.cloudAsrProviderLabel} · 需 Key + 同意（${s.cloudAsrHost || "—"}）`;
    }
  }
  const asrKey = document.querySelector<HTMLButtonElement>("#btnAsrKey");
  const asrConsent = document.querySelector<HTMLButtonElement>("#btnAsrConsent");
  if (asrKey) {
    asrKey.disabled = !s.cloudAsrSupports;
    setQuietButton(asrKey, s.cloudAsrReady, s.cloudAsrReady ? "更换 Key…" : "配置 Key…");
  }
  if (asrConsent) {
    asrConsent.hidden = !s.cloudAsrSupports || s.cloudAsrReady;
    if (!asrConsent.hidden) asrConsent.textContent = "同意上传…";
  }

  const textAi = document.querySelector("#textAiStatus");
  if (textAi) {
    textAi.textContent = s.textAiReady
      ? `已就绪 · ${s.textAiProviderLabel} · ${s.textAiModel}`
      : `未就绪 · ${s.textAiProviderLabel} · 需 Key + 同意（${s.textAiHost || "—"}）`;
  }
  const textAiSummary = document.querySelector("#textAiSummary");
  if (textAiSummary) {
    textAiSummary.textContent = s.textAiReady
      ? `AI 修改已就绪 · ${s.textAiProviderLabel}`
      : "基础转写可直接使用；AI 修改需在高级设置中授权。";
  }
  const textAiSummaryBadge = document.querySelector("#textAiSummaryBadge");
  if (textAiSummaryBadge) {
    textAiSummaryBadge.textContent = s.textAiReady ? "AI 已就绪" : "基础";
    textAiSummaryBadge.className = `badge ${s.textAiReady ? "ok" : ""}`.trim();
  }
  setQuietButton(
    document.querySelector("#btnTextAiKey"),
    s.textAiReady,
    s.textAiReady ? "更换 Key…" : "配置 Key…",
  );
  const textConsent = document.querySelector<HTMLButtonElement>("#btnTextAiConsent");
  if (textConsent) {
    textConsent.hidden = s.textAiReady;
    if (!s.textAiReady) textConsent.textContent = "同意上传…";
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

  const micStatus = document.querySelector("#micStatus");
  if (micStatus) {
    switch (s.microphoneStatus) {
      case "authorized":
        micStatus.textContent = "已授权，可直接录音";
        break;
      case "denied":
        micStatus.textContent = "已拒绝 — 请在系统设置中打开「落字」麦克风";
        break;
      case "restricted":
        micStatus.textContent = "受系统限制，无法使用麦克风";
        break;
      case "notDetermined":
        micStatus.textContent = "尚未询问；首次按住说话时系统会弹出授权";
        break;
      default:
        micStatus.textContent = "无法检测麦克风权限状态";
    }
  }
  setBadge(
    document.querySelector("#micBadge"),
    s.microphoneAuthorized,
    "已就绪",
    s.microphoneStatus === "notDetermined" ? "待确认" : "未授权",
  );
  setQuietButton(
    document.querySelector("#btnOpenMic"),
    s.microphoneAuthorized,
    s.microphoneAuthorized ? "在系统设置中查看" : "打开系统设置",
  );

  const ax = document.querySelector("#axStatus");
  if (ax) {
    ax.textContent = s.accessibilityTrusted
      ? "已生效，可向其他 App 插入文字（失败则降级剪贴板）"
      : "未生效 — 请打开系统设置并开关一次「落字」";
  }
  setBadge(
    document.querySelector("#axBadge"),
    s.accessibilityTrusted,
    "已就绪",
    "未生效",
  );
  setQuietButton(
    document.querySelector("#btnOpenAx"),
    s.accessibilityTrusted,
    s.accessibilityTrusted ? "在系统设置中查看" : "打开系统设置",
  );
  const adhoc = document.querySelector<HTMLElement>("#axAdhocNote");
  if (adhoc) adhoc.hidden = s.accessibilityTrusted;

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

  document.querySelectorAll<HTMLButtonElement>(".help-btn").forEach((btn) => {
    btn.addEventListener("click", (ev) => {
      ev.stopPropagation();
      const text = btn.dataset.help || "";
      const pop = helpPopover();
      if (!pop.hidden && pop.textContent === text) {
        hideHelp();
        return;
      }
      showHelp(btn, text);
    });
  });
  document.addEventListener("click", (ev) => {
    const t = ev.target as HTMLElement | null;
    if (t?.closest(".help-btn") || t?.closest(".help-popover")) return;
    hideHelp();
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

  document.querySelector("#btnPickAsrMode")?.addEventListener("click", () => {
    openPicker({
      title: "选择引擎模式",
      hint: "自动：本地优先，失败再走云端。",
      selectedId: latest?.asrMode,
      options: [
        { id: "auto", label: "自动（本地优先）", detail: "有本地模型就本地，否则云端" },
        { id: "localOnly", label: "仅本地", detail: "不上云，适合离线 / 隐私优先" },
        { id: "cloudOnly", label: "仅云端", detail: "必须配置并同意云端 ASR" },
      ],
      onPick: async (id) => {
        await invoke("settings_set_asr_mode", { mode: id });
        showToast("已切换引擎模式");
      },
    });
  });

  document.querySelector("#btnPickAsrProvider")?.addEventListener("click", () => {
    const providers = latest?.cloudAsrProviders || [];
    openPicker({
      title: "选择云端 ASR 厂商",
      hint: "豆包 Key 格式 APPID:AccessToken；其余为单行 API Key。",
      selectedId: latest?.cloudAsrProvider,
      options: providers.map((p) => ({
        id: p.id,
        label: p.label,
        detail: p.help.slice(0, 56) + "…",
        disabled: !p.supportsCloud,
      })),
      onPick: async (id) => {
        await invoke("settings_set_asr_provider", { providerId: id });
        showToast("已切换云端 ASR 厂商");
      },
    });
  });

  document.querySelector("#btnPickTextProvider")?.addEventListener("click", () => {
    const providers = latest?.textAiProviders || [];
    openPicker({
      title: "选择文本 AI 厂商",
      hint: "豆包模型请填方舟 Endpoint ID（ep-…）。",
      selectedId: latest?.textAiProvider,
      options: providers.map((p) => ({
        id: p.id,
        label: p.label,
        detail: p.help.slice(0, 56) + "…",
      })),
      onPick: async (id) => {
        await invoke("settings_set_text_ai_provider", { providerId: id });
        showToast("已切换文本 AI 厂商");
      },
    });
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
  document.querySelector("#btnTextAiModel")?.addEventListener("click", async () => {
    try {
      await invoke("settings_prompt_text_ai_model");
      await refresh();
      showToast("模型已更新");
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
  let initial = params.get("section") || "general";
  try {
    const pending = await invoke<string | null>("settings_take_nav");
    if (pending) initial = pending;
  } catch {
    /* ignore */
  }
  showSection(initial);

  await refresh();

  const win = getCurrentWindow();
  await win.onFocusChanged(({ payload: focused }) => {
    if (focused) void refresh();
  });
  await win.onCloseRequested(async (event) => {
    event.preventDefault();
    syncPermissionsPoll(false);
    hideHelp();
    await win.hide();
  });
}

main().catch((err) => {
  console.error(err);
  document.body.textContent = `设置页加载失败：${err}`;
});
