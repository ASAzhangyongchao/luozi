import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./overlay.css";

const label = document.querySelector(".label");
const root = document.querySelector(".overlay");
const wave = document.querySelector<HTMLElement>("#wave");
const mark = document.querySelector<HTMLElement>(".mark");
const pulse = document.querySelector<HTMLElement>(".pulse");
const btnCancel = document.querySelector<HTMLButtonElement>("#btnCancel");
const btnConfirm = document.querySelector<HTMLButtonElement>("#btnConfirm");

/** Phases that keep the HUD up until the session ends. */
const STICKY = new Set(["recording", "recording_edit", "transcribing", "delivering"]);
const INTERACTIVE = new Set(["recording", "recording_edit"]);
const AUTO_HIDE_MS = 2200;

let hideTimer: ReturnType<typeof setTimeout> | undefined;

function setControls(phase: string) {
  const live = INTERACTIVE.has(phase);
  if (btnCancel) btnCancel.hidden = !live;
  if (btnConfirm) btnConfirm.hidden = !live;
  if (wave) wave.hidden = !live;
  if (mark) mark.hidden = live;
  if (pulse) pulse.hidden = live;
  root?.classList.toggle("interactive", live);
}

function setPhase(phase: string, message: string) {
  if (!label || !root) return;
  label.textContent = message || phase;
  root.setAttribute("data-phase", phase);
  setControls(phase);
}

function scheduleAutoHide(phase: string) {
  if (hideTimer !== undefined) {
    clearTimeout(hideTimer);
    hideTimer = undefined;
  }
  if (STICKY.has(phase)) return;
  hideTimer = setTimeout(() => {
    void getCurrentWindow().hide();
  }, AUTO_HIDE_MS);
}

btnCancel?.addEventListener("click", async (ev) => {
  ev.stopPropagation();
  try {
    await invoke("session_cancel");
  } catch (err) {
    console.error("session_cancel failed", err);
  }
});

btnConfirm?.addEventListener("click", async (ev) => {
  ev.stopPropagation();
  try {
    await invoke("session_stop");
  } catch (err) {
    console.error("session_stop failed", err);
  }
});

listen<{ phase: string; message: string }>("session://phase", (event) => {
  const { phase, message } = event.payload;
  setPhase(phase, message);
  scheduleAutoHide(phase);
}).catch(console.error);
