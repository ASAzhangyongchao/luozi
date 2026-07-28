import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const source = readFileSync(resolve(process.cwd(), "src-tauri/src/lib.rs"), "utf8");

function trayMenuSource(): string {
  const menuStart = source.indexOf("let menu = Menu::with_items");
  const menuEnd = source.indexOf("let mut tray = TrayIconBuilder", menuStart);

  expect(menuStart).toBeGreaterThanOrEqual(0);
  expect(menuEnd).toBeGreaterThan(menuStart);

  return source.slice(menuStart, menuEnd);
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
    expect(source).toContain('"Luozi 已就绪"');
    expect(source).toContain('"开始语音输入"');
    expect(source).not.toContain("本地+云端（M5）");
    expect(source).not.toContain("M8");
  });

  it("moves engine switching out of the top-level tray menu", () => {
    const menu = trayMenuSource();
    const handlerStart = source.indexOf(".on_menu_event");
    const handlerEnd = source.indexOf(".on_tray_icon_event", handlerStart);
    const handler = source.slice(handlerStart, handlerEnd);

    expect(menu).not.toContain("&asr_mode");
    expect(handler).not.toContain('"asr_mode"');
    expect(source).toContain("fn settings_cycle_asr_mode");
  });
});
