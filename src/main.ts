import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles.css";

type DraftState = {
  text: string;
  canUndo: boolean;
  canRedo: boolean;
  hasLastTranscript: boolean;
  saveStatus: string;
};

type AppConfig = {
  continueSpeakingShortcut: string;
  voiceEditShortcut: string;
  shortcutsProvisional: boolean;
};

type HistoryItem = {
  id: string;
  text: string;
  createdAt: number;
  preview: string;
};

let saveTimer: ReturnType<typeof setTimeout> | undefined;
let applyingRemote = false;
let currentNav: "draft" | "history" = "draft";

const editor = () => document.querySelector<HTMLTextAreaElement>("#editor")!;
const saveHint = () => document.querySelector<HTMLParagraphElement>("#saveHint")!;
const statusEl = () => document.querySelector<HTMLParagraphElement>("#status")!;
const btnUndo = () => document.querySelector<HTMLButtonElement>("#btnUndo")!;
const btnRedo = () => document.querySelector<HTMLButtonElement>("#btnRedo")!;
const btnLoadLast = () => document.querySelector<HTMLButtonElement>("#btnLoadLast")!;
const draftPane = () => document.querySelector<HTMLElement>("#draftPane")!;
const historyPane = () => document.querySelector<HTMLElement>("#historyPane")!;
const draftActions = () => document.querySelector<HTMLElement>("#draftActions")!;
const draftFooterHint = () => document.querySelector<HTMLElement>("#draftFooterHint")!;
const wbTitle = () => document.querySelector<HTMLElement>("#wbTitle")!;
const historyList = () => document.querySelector<HTMLElement>("#historyList")!;
const historyEmpty = () => document.querySelector<HTMLElement>("#historyEmpty")!;

function formatTime(secs: number): string {
  try {
    return new Date(secs * 1000).toLocaleString("zh-CN", {
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return String(secs);
  }
}

function setRailActive(nav: "draft" | "history") {
  document.querySelectorAll<HTMLButtonElement>(".rail-nav .rail-item").forEach((btn) => {
    const on = btn.dataset.nav === nav;
    btn.classList.toggle("active", on);
    if (on) btn.setAttribute("aria-current", "page");
    else btn.removeAttribute("aria-current");
  });
}

async function showDraftView() {
  currentNav = "draft";
  setRailActive("draft");
  draftPane().hidden = false;
  historyPane().hidden = true;
  draftActions().hidden = false;
  draftFooterHint().hidden = false;
  wbTitle().textContent = "语音草稿";
  await getCurrentWindow().setTitle("落字 · 语音草稿");
}

function openSettingsModal(section = "general") {
  const dialog = document.querySelector<HTMLDialogElement>("#settingsDialog");
  const frame = document.querySelector<HTMLIFrameElement>("#settingsFrame");
  if (!dialog || !frame) return;
  frame.src = `./settings.html?embed=1&section=${encodeURIComponent(section)}`;
  if (!dialog.open) dialog.showModal();
}

function closeSettingsModal() {
  const dialog = document.querySelector<HTMLDialogElement>("#settingsDialog");
  dialog?.close();
}

async function refreshHistory() {
  const items = await invoke<HistoryItem[]>("history_list");
  const list = historyList();
  list.replaceChildren();
  historyEmpty().hidden = items.length > 0;
  for (const item of items) {
    const row = document.createElement("article");
    row.className = "history-item";
    row.setAttribute("role", "listitem");

    const meta = document.createElement("div");
    meta.className = "history-item-meta";
    meta.textContent = formatTime(item.createdAt);

    const body = document.createElement("p");
    body.className = "history-item-text";
    body.textContent = item.preview || item.text;

    const actions = document.createElement("div");
    actions.className = "history-item-actions";

    const btnCopy = document.createElement("button");
    btnCopy.type = "button";
    btnCopy.textContent = "复制";
    btnCopy.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(item.text);
        statusEl().textContent = "已复制历史条目";
      } catch {
        statusEl().textContent = "复制失败";
      }
    });

    const btnLoad = document.createElement("button");
    btnLoad.type = "button";
    btnLoad.className = "primary-ghost";
    btnLoad.textContent = "载入草稿";
    btnLoad.addEventListener("click", async () => {
      try {
        applyState(await invoke("history_load_into_draft", { id: item.id }));
        await showDraftView();
        statusEl().textContent = "已载入到草稿";
      } catch (err) {
        statusEl().textContent = `载入失败：${err}`;
      }
    });

    actions.append(btnCopy, btnLoad);
    row.append(meta, body, actions);
    list.append(row);
  }
}

async function showHistoryView() {
  currentNav = "history";
  setRailActive("history");
  draftPane().hidden = true;
  historyPane().hidden = false;
  draftActions().hidden = true;
  draftFooterHint().hidden = true;
  wbTitle().textContent = "落字历史";
  await getCurrentWindow().setTitle("落字 · 落字历史");
  try {
    await refreshHistory();
    statusEl().textContent = "历史 · 本机保存";
  } catch (err) {
    statusEl().textContent = `加载历史失败：${err}`;
  }
}

/** Convert UTF-16 textarea offset → UTF-8 byte offset for Rust string slicing. */
function utf8ByteOffset(text: string, utf16Index: number): number {
  const encoder = new TextEncoder();
  let u16 = 0;
  let bytes = 0;
  for (const ch of text) {
    if (u16 >= utf16Index) break;
    u16 += ch.length;
    bytes += encoder.encode(ch).length;
  }
  return bytes;
}

function reportSelection() {
  const el = editor();
  const start = utf8ByteOffset(el.value, el.selectionStart);
  const end = utf8ByteOffset(el.value, el.selectionEnd);
  void invoke("draft_set_selection", { start, end });
}

function applyState(st: DraftState) {
  applyingRemote = true;
  const el = editor();
  if (el.value !== st.text) {
    const start = el.selectionStart;
    const end = el.selectionEnd;
    el.value = st.text;
    const max = el.value.length;
    el.setSelectionRange(Math.min(start, max), Math.min(end, max));
  }
  btnUndo().disabled = !st.canUndo;
  btnRedo().disabled = !st.canRedo;
  btnLoadLast().hidden = !st.hasLastTranscript;
  applyingRemote = false;
  reportSelection();
}

async function refresh() {
  const st = await invoke<DraftState>("draft_state");
  applyState(st);
  return st;
}

function scheduleSave() {
  if (applyingRemote) return;
  saveHint().textContent = "保存中…";
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    try {
      const st = await invoke<DraftState>("draft_save", { text: editor().value });
      applyState(st);
      saveHint().textContent = "已自动保存";
      setTimeout(() => {
        if (saveHint().textContent === "已自动保存") saveHint().textContent = "";
      }, 1500);
    } catch (err) {
      saveHint().textContent = `保存失败：${err}`;
    }
  }, 1000);
}

async function flushNow() {
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = undefined;
  }
  try {
    await invoke<DraftState>("draft_save", { text: editor().value });
  } catch (err) {
    console.error(err);
  }
}

async function main() {
  const spike = new URLSearchParams(location.search).has("spike");
  const spikePanel = document.querySelector<HTMLElement>("#spikePanel");
  if (spike && spikePanel) {
    spikePanel.hidden = false;
  }

  try {
    const cfg = await invoke<AppConfig>("get_app_config");
    const hint = document.querySelector("#hotkeyHint");
    if (hint) hint.textContent = cfg.continueSpeakingShortcut;
    const editHint = document.querySelector("#editHotkeyHint");
    if (editHint) editHint.textContent = cfg.voiceEditShortcut;
  } catch {
    /* ignore */
  }

  await refresh();

  editor().addEventListener("input", () => {
    scheduleSave();
    reportSelection();
  });
  editor().addEventListener("keyup", () => reportSelection());
  editor().addEventListener("mouseup", () => reportSelection());
  editor().addEventListener("select", () => reportSelection());
  editor().addEventListener("focus", () => reportSelection());

  btnUndo().addEventListener("click", async () => {
    await flushNow();
    applyState(await invoke("draft_undo"));
  });
  btnRedo().addEventListener("click", async () => {
    await flushNow();
    applyState(await invoke("draft_redo"));
  });
  document.querySelector("#btnCopy")!.addEventListener("click", async () => {
    const text = editor().value;
    try {
      await navigator.clipboard.writeText(text);
      statusEl().textContent = "已复制纯文本";
    } catch {
      statusEl().textContent = "复制失败";
    }
  });
  document.querySelector("#btnClear")!.addEventListener("click", async () => {
    if (!editor().value) return;
    if (!confirm("清空当前草稿？此操作可在清空前已保存的版本中恢复需自行备份。")) return;
    applyState(await invoke("draft_clear"));
    statusEl().textContent = "草稿已清空";
  });
  btnLoadLast().addEventListener("click", async () => {
    try {
      applyState(await invoke("draft_load_last"));
      statusEl().textContent = "已载入最近落字";
    } catch {
      statusEl().textContent = "没有可载入的最近落字";
    }
  });

  document.querySelector("#btnRailSettings")?.addEventListener("click", () => {
    openSettingsModal("general");
  });
  document.querySelector("#btnCloseSettings")?.addEventListener("click", () => {
    closeSettingsModal();
  });
  document.querySelector("#settingsDialog")?.addEventListener("close", () => {
    const frame = document.querySelector<HTMLIFrameElement>("#settingsFrame");
    if (frame) frame.src = "about:blank";
  });
  document.querySelector("#btnRailGuide")?.addEventListener("click", async () => {
    try {
      await invoke("open_guide_window");
    } catch (err) {
      statusEl().textContent = `打开教程失败：${err}`;
    }
  });
  document.querySelector<HTMLButtonElement>('[data-nav="draft"]')?.addEventListener("click", () => {
    void showDraftView();
  });
  document.querySelector("#btnRailHistory")?.addEventListener("click", () => {
    void showHistoryView();
  });

  await listen("draft://updated", async (ev) => {
    await refresh();
    if (currentNav === "history") {
      await refreshHistory();
    }
    const reason = (ev.payload as { reason?: string } | null)?.reason;
    statusEl().textContent =
      reason === "voice_edit" ? "已修改" : reason === "load_last" ? "已载入最近落字" : "已写入草稿";
  });

  await listen<{
    reasons: string[];
    originalPreview: string;
    proposedPreview: string;
  }>("draft://edit-preview", async (ev) => {
    const { reasons, originalPreview, proposedPreview } = ev.payload;
    const ok = confirm(
      `高风险修改（${reasons.join(", ")}）\n\n原文：${originalPreview}\n\n改为：${proposedPreview}\n\n应用此修改？`,
    );
    if (ok) {
      applyState(await invoke("draft_apply_pending"));
      statusEl().textContent = "已应用修改";
    } else {
      await invoke("draft_reject_pending");
      statusEl().textContent = "已取消修改";
    }
  });

  await listen("luozi://show-spike", async () => {
    const spikePanel = document.querySelector<HTMLElement>("#spikePanel");
    if (spikePanel) spikePanel.hidden = false;
    statusEl().textContent = "已打开开发 Spike 面板";
  });

  const win = getCurrentWindow();
  await win.onCloseRequested(async (event) => {
    event.preventDefault();
    await flushNow();
    await win.hide();
  });

  statusEl().textContent = "就绪 · 关闭窗口回到托盘";
}

main().catch((err) => {
  console.error(err);
  document.body.textContent = `工作台加载失败：${err}`;
});
