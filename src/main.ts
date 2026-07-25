import {
  isRegistered,
  register,
  unregisterAll,
  type ShortcutEvent,
} from "@tauri-apps/plugin-global-shortcut";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Window } from "@tauri-apps/api/window";
import "./styles.css";

type AppConfig = {
  schemaVersion: number;
  continueSpeakingShortcut: string;
  voiceEditShortcut: string;
  holdToTalk: boolean;
  shortcutsProvisional: boolean;
  language: string;
};

type FocusProbeReport = {
  ok: boolean;
  message: string;
  [key: string]: unknown;
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

type AudioProbeResult = {
  sampleRate: number;
  channels: number;
  frames: number;
  bytes: number;
  deleted: boolean;
};

async function main() {
  const status = document.querySelector<HTMLParagraphElement>("#status")!;
  const shortcutList = document.querySelector<HTMLUListElement>("#shortcutList")!;
  const aboutCard = document.querySelector<HTMLElement>("#aboutCard")!;
  const aboutText = document.querySelector<HTMLPreElement>("#aboutText")!;
  const closeAbout = document.querySelector<HTMLButtonElement>("#closeAbout")!;
  const spikeCard = document.querySelector<HTMLElement>("#spikeCard")!;

  const config = await invoke<AppConfig>("get_app_config");
  shortcutList.innerHTML = `
    <li>继续说：<code>${config.continueSpeakingShortcut}</code></li>
    <li>语音修改：<code>${config.voiceEditShortcut}</code></li>
    <li>按住说话：${config.holdToTalk ? "开" : "关"}</li>
    <li>schemaVersion：${config.schemaVersion}</li>
  `;

  aboutText.textContent = [
    "落字 Luozi  0.0.0",
    `schemaVersion=${config.schemaVersion}`,
    `shortcutsProvisional=${config.shortcutsProvisional}`,
    `continue=${config.continueSpeakingShortcut}`,
    `voiceEdit=${config.voiceEditShortcut}`,
    "M0 Overall=Partial · Mac-first 受限 M1",
    "尚无可下载 Release",
  ].join("\n");

  closeAbout.addEventListener("click", () => {
    aboutCard.hidden = true;
  });

  await listen("luozi://show-about", () => {
    aboutCard.hidden = false;
  });

  const spikeEnabled =
    typeof window !== "undefined" &&
    new URLSearchParams(window.location.search).has("spike");

  if (!spikeEnabled) {
    status.textContent = "练习窗 · 托盘可打开关于 / 退出";
    return;
  }

  spikeCard.hidden = false;
  status.textContent = "Spike 模式（开发）";

  const shortcut = document.querySelector<HTMLSelectElement>("#shortcut")!;
  const registerButton = document.querySelector<HTMLButtonElement>("#register")!;
  const clearButton = document.querySelector<HTMLButtonElement>("#clear")!;
  const autoFocusButton = document.querySelector<HTMLButtonElement>("#autoFocus")!;
  const audioProbeButton = document.querySelector<HTMLButtonElement>("#audioProbe")!;
  const showOverlayButton = document.querySelector<HTMLButtonElement>("#showOverlay")!;
  const hideOverlayButton = document.querySelector<HTMLButtonElement>("#hideOverlay")!;
  const events = document.querySelector<HTMLPreElement>("#events")!;
  const focusReport = document.querySelector<HTMLPreElement>("#focusReport")!;
  const deliveryReport = document.querySelector<HTMLPreElement>("#deliveryReport")!;
  const audioReport = document.querySelector<HTMLPreElement>("#audioReport")!;

  let sequence = 0;
  let deliveryRun = 0;
  const overlay = await Window.getByLabel("overlay");
  shortcut.value = config.continueSpeakingShortcut;

  function appendDelivery(line: string) {
    deliveryReport.textContent = `${line}\n${deliveryReport.textContent ?? ""}`;
  }

  async function runDeliveryProbe() {
    const run = ++deliveryRun;
    try {
      const token = await invoke<TargetToken>("capture_target");
      appendDelivery(`capture\t${JSON.stringify(token)}`);
      window.setTimeout(async () => {
        if (run !== deliveryRun) return;
        const validation = await invoke<ValidationState>("validate_target", { token });
        appendDelivery(`validate\t${validation}`);
        if (validation !== "same_target") {
          appendDelivery("clipboard_fallback_expected\tno_insert");
          return;
        }
        const delivered = await invoke<ValidationState>("deliver_probe", { token });
        appendDelivery(`deliver\t${delivered}`);
      }, 2_000);
    } catch (error) {
      appendDelivery(`target_error\t${String(error)}`);
    }
  }

  function appendEvent(event: ShortcutEvent) {
    sequence += 1;
    events.textContent = `${sequence}\t${Date.now()}\t${event.shortcut}\t${event.state}\n${events.textContent ?? ""}`;
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
    status.textContent = `Registered: ${await isRegistered(candidate)}`;
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
  });
  autoFocusButton.addEventListener("click", async () => {
    focusReport.textContent = "Running…";
    try {
      const report = await invoke<FocusProbeReport>("run_focus_abc_probe");
      focusReport.textContent = JSON.stringify(report, null, 2);
    } catch (error) {
      focusReport.textContent = String(error);
    }
  });
  audioProbeButton.addEventListener("click", async () => {
    audioReport.textContent = "Recording 1s…";
    try {
      audioReport.textContent = JSON.stringify(
        await invoke<AudioProbeResult>("record_one_second_probe"),
        null,
        2,
      );
    } catch (error) {
      audioReport.textContent = String(error);
    }
  });
  showOverlayButton.addEventListener("click", () => void overlay?.show());
  hideOverlayButton.addEventListener("click", () => void overlay?.hide());

  window.addEventListener("beforeunload", () => {
    void unregisterAll();
  });
}

void main();
