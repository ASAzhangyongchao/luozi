import { listen } from "@tauri-apps/api/event";
import "./overlay.css";

const label = document.querySelector(".label");
const root = document.querySelector(".overlay");

function setPhase(phase: string, message: string) {
  if (!label || !root) return;
  label.textContent = message || phase;
  root.setAttribute("data-phase", phase);
}

listen<{ phase: string; message: string }>("session://phase", (event) => {
  setPhase(event.payload.phase, event.payload.message);
}).catch(console.error);
