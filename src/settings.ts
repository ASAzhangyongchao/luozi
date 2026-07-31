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

type InitOptions = {
  root?: ParentNode;
  embedded?: boolean;
  initialSection?: string;
  bindWindowClose?: boolean;
};

type SettingsRootState = {
  latest: SettingsSnapshot | null;
  permissionsPoll?: ReturnType<typeof setInterval>;
};

const boundRoots = new WeakSet<ParentNode>();
const rootStates = new WeakMap<ParentNode, SettingsRootState>();
let helpDocumentBound = false;

function stateFor(root: ParentNode): SettingsRootState {
  let state = rootStates.get(root);
  if (!state) {
    state = { latest: null };
    rootStates.set(root, state);
  }
  return state;
}

function helpPopover() {
  let popover = document.querySelector<HTMLDivElement>("#helpPopover");
  if (!popover) {
    popover = document.createElement("div");
    popover.id = "helpPopover";
    popover.className = "help-popover";
    popover.hidden = true;
    popover.setAttribute("role", "tooltip");
    document.body.appendChild(popover);
  }
  return popover;
}

function pickDialog() {
  let dialog = document.querySelector<HTMLDialogElement>("#pickDialog");
  if (!dialog) {
    dialog = document.createElement("dialog");
    dialog.id = "pickDialog";
    dialog.className = "pick-dialog";
    dialog.innerHTML = `
      <form method="dialog" id="pickForm">
        <h2 id="pickTitle">选择</h2>
        <p class="muted" id="pickHint"></p>
        <div id="pickOptions" class="pick-options"></div>
        <div class="pick-actions">
          <button type="submit" value="cancel" class="btn-quiet">取消</button>
        </div>
      </form>`;
    document.body.appendChild(dialog);
  }
  return dialog;
}

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

function showSection(root: ParentNode, id: string) {
  hideHelp();
  root.querySelectorAll<HTMLElement>(".settings-section").forEach((el) => {
    el.hidden = el.id !== `section-${id}`;
  });
  root.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.section === id);
  });
  syncPermissionsPoll(root, id === "permissions");
  if (id === "permissions") void refresh(root);
}

function syncPermissionsPoll(root: ParentNode, active: boolean) {
  const state = stateFor(root);
  if (state.permissionsPoll !== undefined) {
    clearInterval(state.permissionsPoll);
    state.permissionsPoll = undefined;
  }
  if (!active) return;
  state.permissionsPoll = setInterval(() => {
    void refresh(root);
  }, 1600);
}

function openPicker(root: ParentNode, opts: {
  title: string;
  hint: string;
  options: { id: string; label: string; detail?: string; disabled?: boolean }[];
  selectedId?: string;
  onPick: (id: string) => Promise<void>;
}) {
  const dialog = pickDialog();
  const title = dialog.querySelector("#pickTitle")!;
  const hint = dialog.querySelector("#pickHint")!;
  const box = dialog.querySelector("#pickOptions")!;
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
        await refresh(root);
      } catch (e) {
        if (String(e).includes("canceled")) return;
        showToast(root, String(e));
      }
    });
    box.appendChild(btn);
  }
  dialog.showModal();
}

function applySnapshot(root: ParentNode, s: SettingsSnapshot) {
  stateFor(root).latest = s;
  const asrHint = root.querySelector("#asrModeHint");
  if (asrHint) asrHint.textContent = s.asrModeLabel;
  const modelStatus = root.querySelector("#modelStatus");
  if (modelStatus) modelStatus.textContent = s.modelStatus;
  const modelBadge = root.querySelector("#modelBadge");
  if (modelBadge) {
    modelBadge.textContent = s.modelReady ? "就绪" : "未就绪";
    modelBadge.className = `badge ${s.modelReady ? "ok" : "warn"}`;
  }
  const cloud = root.querySelector("#cloudAsrStatus");
  if (cloud) {
    if (!s.cloudAsrSupports) {
      cloud.textContent = `${s.cloudAsrProviderLabel} · 暂无云端 ASR（请用本地或其他厂商）`;
    } else if (s.cloudAsrReady) {
      cloud.textContent = `已就绪 · ${s.cloudAsrProviderLabel} · ${s.cloudAsrModel || s.cloudAsrHost}`;
    } else {
      cloud.textContent = `未就绪 · ${s.cloudAsrProviderLabel} · 需 Key + 同意（${s.cloudAsrHost || "—"}）`;
    }
  }
  const asrKey = root.querySelector<HTMLButtonElement>("#btnAsrKey");
  const asrConsent = root.querySelector<HTMLButtonElement>("#btnAsrConsent");
  if (asrKey) {
    asrKey.disabled = !s.cloudAsrSupports;
    setQuietButton(asrKey, s.cloudAsrReady, s.cloudAsrReady ? "更换 Key…" : "配置 Key…");
  }
  if (asrConsent) {
    asrConsent.hidden = !s.cloudAsrSupports || s.cloudAsrReady;
    if (!asrConsent.hidden) asrConsent.textContent = "同意上传…";
  }
  const textAi = root.querySelector("#textAiStatus");
  if (textAi) {
    textAi.textContent = s.textAiReady
      ? `已就绪 · ${s.textAiProviderLabel} · ${s.textAiModel}`
      : `未就绪 · ${s.textAiProviderLabel} · 需 Key + 同意（${s.textAiHost || "—"}）`;
  }
  setQuietButton(
    root.querySelector("#btnTextAiKey"),
    s.textAiReady,
    s.textAiReady ? "更换 Key…" : "配置 Key…",
  );
  const textConsent = root.querySelector<HTMLButtonElement>("#btnTextAiConsent");
  if (textConsent) {
    textConsent.hidden = s.textAiReady;
    if (!s.textAiReady) textConsent.textContent = "同意上传…";
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
  const micStatus = root.querySelector("#micStatus");
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
    root.querySelector("#micBadge"),
    s.microphoneAuthorized,
    "已就绪",
    s.microphoneStatus === "notDetermined" ? "待确认" : "未授权",
  );
  setQuietButton(
    root.querySelector("#btnOpenMic"),
    s.microphoneAuthorized,
    s.microphoneAuthorized ? "在系统设置中查看" : "打开系统设置",
  );
  const ax = root.querySelector("#axStatus");
  if (ax) {
    ax.textContent = s.accessibilityTrusted
      ? "已生效，可向其他 App 插入文字（失败则降级剪贴板）"
      : "未生效 — 请打开系统设置并开关一次「落字」";
  }
  setBadge(
    root.querySelector("#axBadge"),
    s.accessibilityTrusted,
    "已就绪",
    "未生效",
  );
  setQuietButton(
    root.querySelector("#btnOpenAx"),
    s.accessibilityTrusted,
    s.accessibilityTrusted ? "在系统设置中查看" : "打开系统设置",
  );
  const adhoc = root.querySelector<HTMLElement>("#axAdhocNote");
  if (adhoc) adhoc.hidden = s.accessibilityTrusted;
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

  root.querySelectorAll<HTMLButtonElement>(".help-btn").forEach((btn) => {
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
  if (!helpDocumentBound) {
    document.addEventListener("click", (ev) => {
      const t = ev.target as HTMLElement | null;
      if (t?.closest(".help-btn") || t?.closest(".help-popover")) return;
      hideHelp();
    });
    helpDocumentBound = true;
  }

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

  root.querySelector("#btnPickAsrMode")?.addEventListener("click", () => {
    const latest = stateFor(root).latest;
    openPicker(root, {
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
        showToast(root, "已切换引擎模式");
      },
    });
  });

  root.querySelector("#btnPickAsrProvider")?.addEventListener("click", () => {
    const latest = stateFor(root).latest;
    const providers = latest?.cloudAsrProviders || [];
    openPicker(root, {
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
        showToast(root, "已切换云端 ASR 厂商");
      },
    });
  });

  root.querySelector("#btnPickTextProvider")?.addEventListener("click", () => {
    const latest = stateFor(root).latest;
    const providers = latest?.textAiProviders || [];
    openPicker(root, {
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
        showToast(root, "已切换文本 AI 厂商");
      },
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
  root.querySelector("#btnTextAiModel")?.addEventListener("click", async () => {
    try {
      await invoke("settings_prompt_text_ai_model");
      await refresh(root);
      showToast(root, "模型已更新");
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
    await win.onFocusChanged(({ payload: focused }) => {
      if (focused) void refresh(root);
    });
    await win.onCloseRequested(async (event) => {
      event.preventDefault();
      syncPermissionsPoll(root, false);
      hideHelp();
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
