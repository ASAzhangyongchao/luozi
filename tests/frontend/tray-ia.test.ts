import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const source = readFileSync(resolve(process.cwd(), "src-tauri/src/lib.rs"), "utf8");
const modelStoreSource = readFileSync(
  resolve(process.cwd(), "src-tauri/src/session/model_store.rs"),
  "utf8",
);
const settingsSource = readFileSync(
  resolve(process.cwd(), "src-tauri/src/session/settings_api.rs"),
  "utf8",
);

function trayMenuSource(): string {
  const menuStart = source.indexOf("let menu = Menu::with_items");
  const menuEnd = source.indexOf("let mut tray = TrayIconBuilder", menuStart);

  expect(menuStart).toBeGreaterThanOrEqual(0);
  expect(menuEnd).toBeGreaterThan(menuStart);

  return source.slice(menuStart, menuEnd);
}

function buildTraySource(): string {
  const start = source.indexOf("fn build_tray");
  const end = source.indexOf("fn register_session_shortcuts", start);

  expect(start).toBeGreaterThanOrEqual(0);
  expect(end).toBeGreaterThan(start);

  return source.slice(start, end);
}

describe("tray information architecture", () => {
  it("places dictation controls before secondary windows and settings", () => {
    const menu = trayMenuSource();
    const itemIds = [...menu.matchAll(/&([a-z_]+),/g)].map((match) => match[1]);

    expect(itemIds).toEqual([
      "title",
      "status",
      "sep_status",
      "start",
      "cancel",
      "undo",
      "sep_actions",
      "draft",
      "voice_edit",
      "sep_prefs",
      "engine",
      "fetch_model",
      "settings",
      "about",
      "quit",
    ]);
  });

  it("uses product language instead of milestone language", () => {
    const tray = buildTraySource();

    expect(tray).toContain('"Luozi 已就绪"');
    expect(tray).toContain('"开始语音输入"');
    expect(tray).toContain('"云端已就绪"');
    expect(tray).toContain('"云端未配置"');
    expect(tray).toContain('"辅助功能已开启"');
    expect(tray).toContain('"辅助功能未开启"');
    expect(tray).not.toContain("Whisper");
    expect(tray).not.toContain("Groq");
    expect(tray).not.toContain("OK");
    expect(tray).not.toContain("M8");

    expect(modelStoreSource).toContain('"本地模型已就绪"');
    expect(modelStoreSource).toContain('"本地模型未安装"');
    expect(modelStoreSource).toContain('"本地模型需重新下载"');
  });

  it("moves engine switching out of the top-level tray menu", () => {
    const menu = trayMenuSource();
    const handlerStart = source.indexOf(".on_menu_event");
    const handlerEnd = source.indexOf(".on_tray_icon_event", handlerStart);
    const handler = source.slice(handlerStart, handlerEnd);

    expect(menu).not.toContain("&asr_mode");
    expect(handler).not.toContain('"asr_mode"');

    const commands = source.slice(source.indexOf(".invoke_handler"));
    expect(commands).toContain("settings_cycle_asr_mode,");
  });

  it("shows only shortcuts that were actually registered", () => {
    const tray = buildTraySource();
    const setup = source.slice(source.indexOf(".setup(|app|"));

    expect(tray).toContain("registered_shortcuts");
    expect(tray).not.toContain("Some(cfg.continue_speaking_shortcut.as_str())");
    expect(tray).not.toContain("Some(cfg.voice_edit_shortcut.as_str())");
    expect(setup.indexOf("register_session_shortcuts")).toBeLessThan(
      setup.indexOf("build_tray"),
    );
  });

  it("shares verified model readiness with settings", () => {
    const tray = buildTraySource();

    expect(tray).toContain("model_store::model_readiness()");
    expect(tray).not.toContain("default_model_path().is_file()");
    expect(settingsSource).toContain("model_store::model_readiness()");
    expect(settingsSource).not.toContain("default_model_path().is_file()");
  });
});
