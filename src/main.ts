import {
  isRegistered,
  register,
  unregisterAll,
  type ShortcutEvent,
} from "@tauri-apps/plugin-global-shortcut";
import { invoke } from "@tauri-apps/api/core";
import { Window } from "@tauri-apps/api/window";
import "./styles.css";

type FocusProbeReport = {
  ok: boolean;
  accessibilityTrusted: boolean;
  textEditActivated: boolean;
  typedValue: string;
  expectedValue: string;
  overlayShowStoleFrontmost: boolean;
  frontmostBeforeOverlay: string;
  frontmostDuringOverlay: string;
  frontmostAfterOverlay: string;
  typingOk: boolean;
  focusOk: boolean;
  message: string;
};

type TargetToken = {
  platform: string;
  processId: number;
  windowId: string;
  elementId: string;
  role: string;
  isSecure: boolean;
  capturedAtMs: number;
};

type ValidationState =
  | "same_target"
  | "changed"
  | "unsupported"
  | "secure";

async function main() {
  const shortcut = document.querySelector<HTMLSelectElement>("#shortcut")!;
  const registerButton = document.querySelector<HTMLButtonElement>("#register")!;
  const clearButton = document.querySelector<HTMLButtonElement>("#clear")!;
  const autoFocusButton = document.querySelector<HTMLButtonElement>("#autoFocus")!;
  const status = document.querySelector<HTMLParagraphElement>("#status")!;
  const events = document.querySelector<HTMLPreElement>("#events")!;
  const focusReport = document.querySelector<HTMLPreElement>("#focusReport")!;
  const deliveryReport = document.querySelector<HTMLPreElement>("#deliveryReport")!;

  let sequence = 0;
  let deliveryRun = 0;
  const overlay = await Window.getByLabel("overlay");

  shortcut.value = "Control+Alt+Space";

  function appendDelivery(line: string) {
    deliveryReport.textContent = `${line}\n${deliveryReport.textContent ?? ""}`;
  }

  async function runDeliveryProbe() {
    const run = ++deliveryRun;

    try {
      const token = await invoke<TargetToken>("capture_target");
      appendDelivery(`capture\t${JSON.stringify(token)}`);
      events.textContent =
        `capture\t${JSON.stringify(token)}\n${events.textContent ?? ""}`;

      window.setTimeout(async () => {
        if (run !== deliveryRun) return;

        const validation = await invoke<ValidationState>("validate_target", {
          token,
        });
        appendDelivery(`validate\t${validation}`);
        events.textContent =
          `validate\t${validation}\n${events.textContent ?? ""}`;

        if (validation !== "same_target") {
          appendDelivery("clipboard_fallback_expected\tno_insert");
          return;
        }

        const delivered = await invoke<ValidationState>("deliver_probe", {
          token,
        });
        appendDelivery(`deliver\t${delivered}`);
        events.textContent =
          `deliver\t${delivered}\n${events.textContent ?? ""}`;

        if (delivered === "unsupported" || delivered === "changed" || delivered === "secure") {
          appendDelivery("clipboard_fallback_expected\tno_insert");
        } else if (delivered === "same_target") {
          appendDelivery("direct_write\tok");
        }
      }, 2_000);
    } catch (error) {
      appendDelivery(`target_error\t${String(error)}`);
      events.textContent =
        `target_error\t${String(error)}\n${events.textContent ?? ""}`;
    }
  }

  function appendEvent(event: ShortcutEvent) {
    sequence += 1;
    const line = `${sequence}\t${Date.now()}\t${event.shortcut}\t${event.state}`;
    events.textContent = `${line}\n${events.textContent ?? ""}`;

    if (event.state === "Pressed") {
      void overlay?.show();
      void runDeliveryProbe();
    } else {
      void overlay?.hide();
    }
  }

  async function registerCandidate(candidate: string) {
    await unregisterAll();
    events.textContent = "";
    sequence = 0;
    await register(candidate, appendEvent);
    const ownedByThisApp = await isRegistered(candidate);
    status.textContent =
      `Registered by Luozi: ${ownedByThisApp}. ` +
      "Pressed captures target; after 2s validates and may write 落字测试.";
  }

  registerButton.addEventListener("click", async () => {
    try {
      await registerCandidate(shortcut.value);
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

  autoFocusButton.addEventListener("click", async () => {
    focusReport.textContent = "Running focus probe…";
    try {
      await registerCandidate("Control+Alt+Space");
      const report = await invoke<FocusProbeReport>("run_focus_abc_probe");
      focusReport.textContent = JSON.stringify(report, null, 2);
      status.textContent = report.message;
    } catch (error) {
      focusReport.textContent = String(error);
      status.textContent = `Focus probe failed: ${String(error)}`;
    }
  });

  window.addEventListener("beforeunload", () => {
    void unregisterAll();
  });

  try {
    await registerCandidate("Control+Alt+Space");
  } catch (error) {
    status.textContent = `Auto-register failed: ${String(error)}`;
  }
}

void main();
