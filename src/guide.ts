import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles-settings.css";

function showSection(id: string) {
  document.querySelectorAll<HTMLElement>(".settings-section").forEach((el) => {
    el.hidden = el.id !== `section-${id}`;
  });
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.section === id);
  });
}

async function main() {
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.addEventListener("click", () => showSection(btn.dataset.section || "start"));
  });

  document.querySelector("#btnOpenSettings")?.addEventListener("click", async () => {
    try {
      await invoke("open_settings_window", { section: "general" });
    } catch (err) {
      console.error("open settings failed", err);
    }
  });

  // Ensure title if opened via deep link later.
  try {
    await getCurrentWindow().setTitle("落字 · 如何使用");
  } catch {
    /* ignore in plain browser preview */
  }

  showSection("start");
}

void main();
