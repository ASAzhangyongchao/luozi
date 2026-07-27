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
  shortcutsProvisional: boolean;
};

let saveTimer: ReturnType<typeof setTimeout> | undefined;
let applyingRemote = false;

const editor = () => document.querySelector<HTMLTextAreaElement>("#editor")!;
const saveHint = () => document.querySelector<HTMLParagraphElement>("#saveHint")!;
const statusEl = () => document.querySelector<HTMLParagraphElement>("#status")!;
const btnUndo = () => document.querySelector<HTMLButtonElement>("#btnUndo")!;
const btnRedo = () => document.querySelector<HTMLButtonElement>("#btnRedo")!;
const btnLoadLast = () => document.querySelector<HTMLButtonElement>("#btnLoadLast")!;

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
  } catch {
    /* ignore */
  }

  await refresh();

  editor().addEventListener("input", () => scheduleSave());

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

  await listen("draft://updated", async () => {
    await refresh();
    statusEl().textContent = "已写入草稿";
  });

  const win = getCurrentWindow();
  await win.onCloseRequested(async (event) => {
    // Rust also prevents close + flush; this is belt-and-suspenders for pending edits.
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
