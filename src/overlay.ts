import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./overlay.css";

const label = document.querySelector(".label");
const root = document.querySelector(".overlay");

/** Phases that keep the HUD up until the session ends. */
const STICKY = new Set(["recording", "transcribing", "delivering"]);
const AUTO_HIDE_MS = 2200;

let hideTimer: ReturnType<typeof setTimeout> | undefined;

function setPhase(phase: string, message: string) {
  if (!label || !root) return;
  label.textContent = message || phase;
  root.setAttribute("data-phase", phase);
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

listen<{ phase: string; message: string }>("session://phase", (event) => {
  const { phase, message } = event.payload;
  setPhase(phase, message);
  scheduleAutoHide(phase);
}).catch(console.error);
