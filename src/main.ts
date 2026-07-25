import {
  isRegistered,
  register,
  unregisterAll,
  type ShortcutEvent,
} from "@tauri-apps/plugin-global-shortcut";
import "./styles.css";

const shortcut = document.querySelector<HTMLSelectElement>("#shortcut")!;
const registerButton = document.querySelector<HTMLButtonElement>("#register")!;
const clearButton = document.querySelector<HTMLButtonElement>("#clear")!;
const status = document.querySelector<HTMLParagraphElement>("#status")!;
const events = document.querySelector<HTMLPreElement>("#events")!;

let sequence = 0;

function appendEvent(event: ShortcutEvent) {
  sequence += 1;
  const line = `${sequence}\t${Date.now()}\t${event.shortcut}\t${event.state}`;
  events.textContent = `${line}\n${events.textContent ?? ""}`;
}

registerButton.addEventListener("click", async () => {
  await unregisterAll();
  events.textContent = "";
  sequence = 0;
  const candidate = shortcut.value;

  try {
    await register(candidate, appendEvent);
    const ownedByThisApp = await isRegistered(candidate);
    status.textContent =
      `Registered by Luozi: ${ownedByThisApp}. ` +
      "Now perform the guided press/release test.";
  } catch (error) {
    status.textContent = `Registration failed: ${String(error)}`;
  }
});

clearButton.addEventListener("click", async () => {
  await unregisterAll();
  status.textContent = "Not registered";
  events.textContent = "";
  sequence = 0;
});

window.addEventListener("beforeunload", () => {
  void unregisterAll();
});
